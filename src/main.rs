mod gopher;

use anyhow::Result;
use async_std::io::prelude::BufReadExt;

use async_std::stream::StreamExt;
use clap::Parser;
use gopher::GopherItem;
use serde::Deserialize;
use tide::{http::mime, Request};
use tide::{prelude::*, Body};
use tinytemplate::TinyTemplate;
use url::Url;

const _PAGE_HTML: &str = include_str!("../static/page.html");

#[derive(Deserialize)]
struct ProxyReq {
    url: String,
    #[serde(alias = "t", default = "default_type")]
    item_type: char,
}

#[doc(hidden)]
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("localhost:8080"))]
    listen_addr: String,
}

fn default_type() -> char {
    let d = GopherItem::Submenu;
    d.into()
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

    let result = match GopherItem::from(r.item_type) {
        GopherItem::Submenu => render_submenu(url).await,
        GopherItem::TextFile => render_text(url).await,
        t => proxy_file(url, t).await,
    };

    match result {
        Ok(resp) => Ok(resp),
        Err(err) => Ok(tide::Response::builder(200)
            .body(render_page(PageTemplate {
                title: String::from("proxy70"),
                body: format!("<pre>error loading resource: {:} </pre>", err),
            })?)
            .content_type(mime::HTML)
            .build()),
    }
}

async fn proxy_file(url: Url, t: GopherItem) -> tide::Result {
    let response = gopher::fetch_url(url).await?;
    let mut r = Body::from_reader(response, None);
    r.set_mime(t);
    Ok(r.into())
}

async fn render_text(url: Url) -> tide::Result {
    let mut body = String::new();
    body.push_str("<pre>\n");
    let mut lines = gopher::fetch_url(url).await?.lines();

    while let Some(Ok(line)) = lines.next().await {
        if line == "." {
            break;
        }
        body.push_str(&line);
        body.push_str("\n");
    }
    body.push_str("</pre>");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
        })?)
        .content_type(mime::HTML)
        .build())
}

async fn render_submenu(url: Url) -> tide::Result {
    let mut body = String::new();
    let mut response = gopher::fetch_url(url).await?.lines();
    body.push_str("<table>\n");
    let mut paragraph = String::new();
    while let Some(Ok(line)) = response.next().await {
        if line == "." {
            break;
        }

        let entry = gopher::DirEntry::from(line.as_str());

        if entry.item_type == GopherItem::Info {
            // consume any subsequent Info items into single paragraph
            // to make sure pseudographics in menus is shown as intended
            if paragraph.is_empty() {
                paragraph.push_str("<tr><td></td><td><pre>");
            }
            paragraph.push_str(format!("{}\n", &entry.label).as_str());
            continue;
        } else if !paragraph.is_empty() {
            body.push_str(format!("{}</pre></td></tr>", paragraph).as_str());
            paragraph.clear();
        }

        body.push_str("<tr>\n");
        match entry.item_type {
            GopherItem::Unknown => continue,
            GopherItem::Submenu => {
                body.push_str(format!("<td><i class=\"fa fa-folder-o\"></i></td>").as_str());
            }
            GopherItem::TextFile => {
                body.push_str(format!("<td><i class=\"fa fa-file-text-o\"></i></td>").as_str());
            }
            GopherItem::ImageFile => {
                body.push_str(
                    format!(
                        "<td></td><td><img src=\"{}\" />\n</tr>",
                        entry.to_href().unwrap()
                    )
                    .as_str(),
                );
                continue;
            }
            _ => body.push_str("<td></td>"),
        }

        body.push_str("<td><pre>");
        match entry.to_href() {
            Some(href) => body.push_str(&format!("<a href=\"{}\">{}</a>", href, entry.label)),
            None => body.push_str(&format!("{}", entry.label)),
        }
        body.push_str("</pre></td></tr>\n");
    }
    if !paragraph.is_empty() {
        body.push_str(format!("{}</pre></td></tr>", paragraph).as_str());
        paragraph.clear();
    }

    body.push_str("</table>\n");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
        })?)
        .content_type(mime::HTML)
        .build())
}

async fn proxy_redirect(req: Request<()>) -> tide::Result {
    Ok(format!("request to {}", req.url()).into())
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    femme::start();
    let args = Args::parse();

    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/").get(root);
    app.at("/proxy").get(proxy_req);
    app.at("/proxy/*").get(proxy_redirect);
    app.at("/static").serve_dir("static/")?;
    app.listen(args.listen_addr).await?;
    Ok(())
}
