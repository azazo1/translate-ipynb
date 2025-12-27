//! Translate notebook
use std::{borrow::Cow, fs, path::PathBuf};

use clap::Parser;
use regex::{Captures, RegexBuilder};
use serde_json::Value;
use translate_ipynb::TranslateAgent;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    #[clap(short, long, help = "Input notebook path")]
    ipynb: PathBuf,
    #[clap(
        short,
        long,
        default_value = "sk-DlhFc2IaClcNuIr498Ee7dA4Ed1f4a219377471e81165f6d"
    )]
    api_key: String,
    #[clap(short, long, default_value = "https://aihubmix.com/v1")]
    base_url: String,
    #[clap(short, long, default_value = "gpt-4o-mini")]
    model: String,
    #[clap(
        short,
        long,
        help = "Target language translate to",
        default_value = "zh"
    )]
    lang: String,
    #[clap(short, long, help = "Output notebook path")]
    output: String,
}

struct App {
    args: AppArgs,
}

impl App {
    fn new(args: AppArgs) -> Self {
        Self { args }
    }

    async fn run(self) -> anyhow::Result<()> {
        let mut ipynb: Value = serde_json::from_reader(
            fs::OpenOptions::new().read(true).open(&self.args.ipynb)?,
        )?;
        // dbg!(&ipynb);

        let agent = TranslateAgent::new(
            &self.args.api_key,
            &self.args.base_url,
            &self.args.model,
            &self.args.lang,
        )?;

        let doc_comment_pat = RegexBuilder::new(
            r#"
            "{3}
            (.*?)
            "{3}
            "#,
        )
        .dot_matches_new_line(true)
        .ignore_whitespace(true)
        .build()?;

        let mut total_len = 0;
        for (idx, cell) in ipynb
            .as_object_mut()
            .ok_or(anyhow::anyhow!("root not object"))?
            .get_mut("cells")
            .ok_or(anyhow::anyhow!("no cells"))?
            .as_array_mut()
            .inspect(|v| total_len = v.len())
            .ok_or(anyhow::anyhow!("cells not array"))?
            .iter_mut()
            .enumerate()
        {
            let cell_type = cell
                .get("cell_type")
                .ok_or(anyhow::anyhow!("no cell_type"))?
                .as_str()
                .ok_or(anyhow::anyhow!("cell_type not string"))?
                .to_string();

            if cell_type != "markdown" && cell_type != "code" {
                continue;
            }
            let mut source_array = false;
            let source =
                cell.get_mut("source").ok_or(anyhow::anyhow!("no source"))?;
            let mut s = source.as_str().map(|s| s.to_string());
            if s.is_none() {
                let strings = source
                    .as_array()
                    .ok_or(anyhow::anyhow!("source type not supported"))?;
                s = Some(
                    strings
                        .iter()
                        .map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Option<Vec<String>>>()
                        .ok_or(anyhow::anyhow!("source item is not string"))?
                        .join("\n"),
                );
                source_array = true;
            }
            let s = s.unwrap();
            let rst: Cow<str> = if cell_type == "markdown" {
                agent.translate(&s).await?.into()
                // s.into()
            } else {
                let mut comments = Vec::new();
                for cap in doc_comment_pat.captures_iter(&s) {
                    comments.push(cap.get(1).unwrap().as_str().to_string());
                }
                for comm in comments.iter_mut() {
                    *comm = agent.translate(comm).await?;
                }
                let mut cnt = 0;
                doc_comment_pat.replace_all(&s, move |_: &Captures| {
                    let comm = comments[cnt].to_string();
                    cnt += 1;
                    format!(r#""""{comm}""""#)
                })
            };
            println!(
                "{} / {total_len}: {}",
                idx + 1,
                rst.chars().take(10).collect::<String>().replace("\n", " ")
            );
            if source_array {
                *source = Value::Array(
                    rst.lines().map(|l| Value::String(l.to_string())).collect(),
                );
            } else {
                *source = Value::String(rst.to_string());
            }
        }

        fs::write(self.args.output, ipynb.to_string())?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();

    App::new(args).run().await
}
