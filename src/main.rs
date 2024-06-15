use std::net::Shutdown;

use anyhow::{anyhow, Result};
use async_std::io::prelude::BufReadExt;
use async_std::io::{BufReader, ReadExt, WriteExt};
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use serde::Deserialize;
use tide::prelude::*;
use tide::{http::mime, Request};
use tinytemplate::TinyTemplate;
use url::Url;

const _PAGE_HTML: &str = include_str!("page.html");

#[derive(Deserialize)]
struct ProxyReq {
    url: String,
}

#[derive(Serialize)]
struct PageTemplate {
    title: String,
    body: String,
}

fn render_page(tpl: PageTemplate) -> Result<String, anyhow::Error> {
    let mut tt = TinyTemplate::new();
    tt.add_template("page", _PAGE_HTML)?;
    Ok(tt.render("page", &tpl)?)
}

async fn root(mut _req: Request<()>) -> tide::Result {
    let body = String::from(
        r#"
    <form action="proxy" method="get">
        <label for="url">gopher://</label>
        <input name="url" id="url" type="text">
        <input type="submit" value="Go">
    </form>"#,
    );

    let resp = tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
        })?)
        .content_type(mime::HTML)
        .build();
    Ok(resp)
}

async fn proxy_req(req: Request<()>) -> tide::Result {
    let mut r: ProxyReq = req.query()?;
    if !r.url.starts_with("gopher://") {
        r.url = format!("gopher://{}", r.url);
    }

    let mut url = Url::parse(&r.url)?;
    if url.port().is_none() {
        let _ = url.set_port(Some(70));
    }

    let mut gopher_resp = fetch_site(url).await?;
    gopher_resp = gopher_resp.replace("\r\n", "\n<br>\n");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("port70"),
            body: format!("<span>{}<span>", gopher_resp),
        })?)
        .content_type(mime::HTML)
        .build())
}

async fn fetch_site(url: Url) -> Result<String, anyhow::Error> {
    let mut stream = TcpStream::connect(format!(
        "{}:{}",
        url.host().unwrap(),
        url.port().unwrap_or(70),
    ))
    .await?;
    let mut result = String::new();
    stream
        .write_all(format!("{}\r\n", url.path()).as_bytes())
        .await?;
    stream.read_to_string(&mut result).await?;
    // let mut lines = BufReader::new(stream).lines().fuse();
    // loop {
    //     match lines.next().await {
    //         Some(line) => match line?.as_str() {
    //             "." => {
    //                 stream.shutdown(Shutdown::Both)?;
    //                 break;
    //             }
    //             v => {
    //                 result.push_str("\n");
    //                 result.push_str(v);
    //             }
    //         },
    //         None => break,
    //     }
    // }
    Ok(result)
}

async fn proxy_redirect(req: Request<()>) -> tide::Result {
    Ok(format!("request to {}", req.url()).into())
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    femme::start();

    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/").get(root);
    app.at("/proxy").get(proxy_req);
    app.at("/proxy/*").get(proxy_redirect);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
