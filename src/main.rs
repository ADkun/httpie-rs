extern crate clap;
// reqwest 依赖于tokio异步运行时
use reqwest::{header, Client, Response, Url};
use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use std::{str::FromStr, collections::HashMap};
use mime::Mime;
use colored::Colorize;

// 定义 HTTPie 的 CLI 主入口，包含多个命令
// 下面 /// 的注释是文档， clap 会将其当成是 CLI 的帮助

/// A naive httpie implementation wite Rust, can you imagine how easy it is?
#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "adkun")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

/// 子命令分别对应不同的 HTTP 方法，暂时只支持 GET / POST 方法
#[derive(Parser, Debug)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

#[derive(Parser, Debug)]
struct Get {
    #[clap(value_parser=parse_url)]
    url: String,
}

#[derive(Parser, Debug)]
struct Post {
    #[clap(value_parser=parse_url)]
    url: String,
    #[clap(value_parser=parse_kv_pair)]
    body: Vec<KvPair>,
}

#[derive(Debug, Clone, PartialEq)]
struct KvPair {
    k: String,
    v: String,
}


impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 使用 = 进行 split，这会得到一个迭代器
        let mut split = s.split('=');
        let err = || anyhow!(format!("Failed to parse {}", s));
        Ok(Self {
            // 从迭代器中取第一个结果作为 key，迭代器返回 Some(T)/None
            // 我们将其转换成 Ok(T)/Err(E)，然后用 ? 处理错误
            k: (split.next().ok_or_else(err)?).to_string(),
            // 从迭代器中取第二个结果作为 value
            v: (split.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    Ok(s.parse()?)
}

fn parse_url(s: &str) -> Result<String> {
    // check url
    let _url: Url = s.parse()?;

    Ok(s.into())
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;
    Ok(print_resp(resp).await?)
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.k, &pair.v);
    }
    let resp = client.post(&args.url).json(&body).send().await?;
    Ok(print_resp(resp).await?)
}

/// Print server version + HTTP status
fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

/// Print server return's HTTP header
fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }

    print!("\n");
}

/// Print server return's HTTP body
fn print_body(m: Option<Mime>, body: &String) {
    match m {
        // pretty print application/json
        Some(v) if v == mime::APPLICATION_JSON => {
            println!("{}", jsonxf::pretty_print(body).unwrap().cyan())
        }
        _ => println!("{}", body),
    }
}

/// Parse the content-type that server returned to Mime
fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

/// Print the whole response
async fn print_resp(resp: Response) -> Result<()> {
    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(mime, &body);
    Ok(())
}


#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    // Generate a HTTP client
    let mut headers = header::HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust Httpie".parse()?);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let result = match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    };

    Ok(result)
}
