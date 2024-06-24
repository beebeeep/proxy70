mod gopher;

use anyhow::Result;
use async_std::io::prelude::BufReadExt;

use async_std::stream::StreamExt;
use clap::Parser;
use gopher::{GopherItem, GopherURL};
use serde::Deserialize;

use tide::{http::mime, Request};
use tide::{prelude::*, Body};
use tinytemplate::TinyTemplate;
use url::Url;

const _PAGE_HTML: &str = include_str!("../static/page.html");
const _WELCOME_HTML: &str = include_str!("../static/welcome.html");

#[derive(Deserialize)]
struct ProxyReq {
    url: Option<String>,
    #[serde(alias = "t")]
    item_type: Option<char>,
    query: Option<String>,
}

#[doc(hidden)]
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("localhost:8080"))]
    listen_addr: String,
}

#[derive(Serialize)]
struct PageTemplate {
    title: String,
    body: String,
    url: Option<String>,
}

fn render_page(tpl: PageTemplate) -> Result<String, anyhow::Error> {
    let mut tt = TinyTemplate::new();
    tt.add_template("page", _PAGE_HTML)?;
    Ok(tt.render("page", &tpl)?)
}

async fn render_nav(mut _req: Request<()>) -> tide::Result {
    let resp = tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: String::from(_WELCOME_HTML),
            url: None,
        })?)
        .content_type(mime::HTML)
        .build();
    Ok(resp)
}

async fn root(req: Request<()>) -> tide::Result {
    let r: ProxyReq = req.query()?;
    match r.url {
        None => render_nav(req).await,
        Some(mut url_str) => {
            if !url_str.starts_with("gopher://") {
                url_str = format!("gopher://{}", url_str);
            }

            let url = GopherURL::from(url_str.as_str());

            let result = match GopherItem::from(r.item_type.unwrap_or(GopherItem::Submenu.into())) {
                GopherItem::Submenu => render_submenu(&url, None).await,
                GopherItem::FullTextSearch => render_submenu(&url, r.query).await,
                GopherItem::TextFile => render_text(&url).await,
                t => proxy_file(&url, t).await,
            };

            match result {
                Ok(resp) => Ok(resp),
                Err(err) => Ok(tide::Response::builder(200)
                    .body(render_page(PageTemplate {
                        title: String::from("proxy70"),
                        body: format!("<pre>error loading resource: {:} </pre>", err),
                        url: Some(url.to_string()),
                    })?)
                    .content_type(mime::HTML)
                    .build()),
            }
        }
    }
}

async fn proxy_file(url: &GopherURL, t: GopherItem) -> tide::Result {
    let response = gopher::fetch_url(url, None).await?;
    let body = Body::from_reader(response, None);
    let mut builder = tide::Response::builder(200);
    if let Some(filename) = url.selector.split("/").last() {
        builder = builder.header(
            "Content-disposition",
            format!("attachement; filename=\"{}\"", filename),
        );
    }

    Ok(builder.body(body).content_type(t).build())
}

async fn render_text(url: &GopherURL) -> tide::Result {
    let mut body = String::new();
    body.push_str("<pre>\n");
    let mut lines = gopher::fetch_url(&url, None).await?.lines();

    while let Some(Ok(line)) = lines.next().await {
        if line == "." {
            break;
        }
        body.push_str(&html_escape::encode_text(&line));
        body.push_str("\n");
    }
    body.push_str("</pre>");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
            url: Some(url.to_string()),
        })?)
        .content_type(mime::HTML)
        .build())
}

async fn render_submenu(url: &GopherURL, query: Option<String>) -> tide::Result {
    let mut body = String::new();
    let menu = gopher::Menu::from_url(&url, query).await?;
    body.push_str("<table>\n");
    for item in menu.items {
        body.push_str(
            format!("<tr>{}</tr>", item.format_row().unwrap_or(String::from(""))).as_str(),
        )
    }
    body.push_str("</table>\n");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
            url: Some(url.to_string()),
        })?)
        .content_type(mime::HTML)
        .build())
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    femme::start();
    let args = Args::parse();

    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/").get(root);
    app.at("/static").serve_dir("static/")?;

    app.listen(args.listen_addr).await?;
    Ok(())
}
