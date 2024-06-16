mod gopher;

use anyhow::Result;
use async_std::io::prelude::BufReadExt;
use async_std::io::ReadExt;
use async_std::stream::StreamExt;
use gopher::{DirEntry, GopherItem};
use serde::Deserialize;
use tide::{http::mime, Request};
use tide::{log, prelude::*};
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

    let mut body = String::new();
    let mut response = gopher::fetch_url(url).await?.lines();
    let mut is_para = false;
    // body.push_str("<table>\n");
    // while let Some(rline) = response.next().await {
    //     let line = rline.unwrap_or_default();
    //     log::info!("got {}", line);
    //     let entry = gopher::DirEntry::from(line.as_str());
    //     log::info!("parsed as {:?}", entry);
    //     if entry.item_type == GopherItem::Info {
    //         if !is_para {
    //             body.push_str("<p>\n");
    //             is_para = true;
    //         }
    //         body.push_str(&entry.label);
    //         body.push_str("\n");
    //     } else {
    //         if is_para {
    //             body.push_str("</p>\n");
    //             is_para = false
    //         }
    //         body.push_str("<span>\n");
    //         if entry.item_type == GopherItem::Submenu {
    //             body.push_str("<i class=\"fa fa-folder\"></i> ")
    //         }
    //         match entry.url {
    //             Some(url) => {
    //                 body.push_str(&format!("<a href=\"{}\">{}</a><br>\n", url, entry.label))
    //             }
    //             None => body.push_str(&format!("{}<br>\n", entry.label)),
    //         }
    //         body.push_str("</span>\n");
    //     }
    // }
    // body.push_str("</table>\n");
    body.push_str("<table>\n");
    while let Some(rline) = response.next().await {
        body.push_str("<tr>\n");
        let entry = gopher::DirEntry::from(rline.unwrap_or_default().as_str());

        match entry.item_type {
            GopherItem::Submenu => {
                body.push_str(format!("<td><i class=\"fa fa-folder-o\"></i></td>").as_str());
            }
            GopherItem::TextFile => {
                body.push_str(format!("<td><i class=\"fa fa-file-text-o\"></i></td>").as_str());
            }
            _ => body.push_str("<td></td>"),
        }
        body.push_str("<td>");
        match entry.url {
            Some(url) => body.push_str(&format!("<a href=\"{}\">{}</a>\n", url, entry.label)),
            None => body.push_str(&format!("{}\n", entry.label)),
        }
        body.push_str("</td></tr>\n");
    }
    body.push_str("</table>\n");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("port70"),
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

    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/").get(root);
    app.at("/proxy").get(proxy_req);
    app.at("/proxy/*").get(proxy_redirect);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
