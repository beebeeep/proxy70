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
    let body = String::from(
        r#"
    <form action="/" method="get">
        <label for="url">gopher://</label>
        <input name="url" id="url" type="text">
        <input type="submit" value="Go">
    </form>"#,
    );

    let resp = tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
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

            let mut url = Url::parse(&url_str)?;
            if url.port().is_none() {
                let _ = url.set_port(Some(70));
            }

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
                        url: format_gopher_url(&url, GopherItem::Error),
                    })?)
                    .content_type(mime::HTML)
                    .build()),
            }
        }
    }
}

async fn proxy_file(url: &Url, t: GopherItem) -> tide::Result {
    let response = gopher::fetch_url(url, None).await?;
    let body = Body::from_reader(response, None);
    let mut builder = tide::Response::builder(200);
    if let Some(s) = url.path_segments() {
        if let Some(filename) = s.last() {
            builder = builder.header(
                "Content-disposition",
                format!("attachement; filename=\"{}\"", filename),
            );
        }
    }

    Ok(builder.body(body).content_type(t).build())
}

async fn render_text(url: &Url) -> tide::Result {
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
            url: format_gopher_url(&url, GopherItem::TextFile),
        })?)
        .content_type(mime::HTML)
        .build())
}

async fn render_submenu(url: &Url, query: Option<String>) -> tide::Result {
    let mut body = String::new();
    let mut response = gopher::fetch_url(&url, query).await?.lines();
    body.push_str("<table>\n");
    let mut paragraph = String::new();
    while let Some(Ok(line)) = response.next().await {
        if line == "." {
            break;
        }

        let entry = gopher::DirEntry::from(line.as_str());
        let label = html_escape::encode_text(&entry.label);

        if entry.item_type == GopherItem::Info {
            // consume any subsequent Info items into single paragraph
            // to make sure pseudographics in menus is shown as intended
            if paragraph.is_empty() {
                paragraph.push_str("<tr><td></td><td><pre id=\"pre_content\">");
            }
            paragraph.push_str(format!("{}\n", &label).as_str());
            continue;
        } else if !paragraph.is_empty() {
            body.push_str(format!("{}</pre></td></tr>", paragraph).as_str());
            paragraph.clear();
        }

        body.push_str("<tr>\n");
        // draw table row
        match entry.item_type {
            GopherItem::Unknown => continue,
            GopherItem::Submenu => {
                body.push_str(format!("<td><i class=\"fa fa-folder-o\"></i></td>").as_str());
            }
            GopherItem::TextFile => {
                body.push_str(format!("<td><i class=\"fa fa-file-text-o\"></i></td>").as_str());
            }
            GopherItem::HtmlFile => {
                body.push_str(
                    format!(
                        "<td><i class=\"fa fa-external-link\"></i></td><td><a href=\"{}\"><pre>{}</pre></a>",
                        entry.url.unwrap(),
                        label
                    )
                    .as_str(),
                );
                continue;
            }
            GopherItem::WavFile | GopherItem::SoundFile => {
                body.push_str(
                    format!(
                        r#"<td></td><td>
                                <pre>{0} (<a href="{1}">download</a>)</pre>
                                <audio controls><source src="{1}">Your browser does not support audio element.</audio>
                            </td></tr>"#,
                        label,
                        entry.to_href().unwrap(),
                    )
                    .as_str(),
                );
                continue;
            }
            GopherItem::FullTextSearch => {
                body.push_str(
                    format!(
                        r#"<td><i class="fa fa-search"></i></td>
                           <td><form action="/" method="get">
                               <input name="query"  placeholder="{}" type="text">
                               <input type="hidden" name="url" value="{}">
                               <input type="hidden" name="t" value="{}">
                               <input type="submit" value="Submit">
                           </form></td><tr>"#,
                        label,
                        &entry.url.unwrap().to_string(),
                        Into::<char>::into(entry.item_type.clone()),
                    )
                    .as_str(),
                );
                continue;
            }
            GopherItem::ImageFile
            | GopherItem::BitmapFile
            | GopherItem::GifFile
            | GopherItem::PngFile => {
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

        // if we are here, just leave link to referred page
        body.push_str("<td><pre>");
        match entry.to_href() {
            Some(href) => body.push_str(&format!("<a href=\"{}\">{}</a>", href, label)),
            None => body.push_str(&format!("{}", label)),
        }
        body.push_str("</pre></td></tr>\n");
    }

    // mb finalize paragraph
    if !paragraph.is_empty() {
        body.push_str(format!("{}</pre></td></tr>", paragraph).as_str());
        paragraph.clear();
    }

    body.push_str("</table>\n");
    Ok(tide::Response::builder(200)
        .body(render_page(PageTemplate {
            title: String::from("proxy70"),
            body: body,
            url: format_gopher_url(&url, GopherItem::Submenu),
        })?)
        .content_type(mime::HTML)
        .build())
}

fn format_gopher_url(url: &Url, t: GopherItem) -> Option<String> {
    let mut path = match urlencoding::decode(url.path()) {
        Ok(p) => p.into_owned(),
        Err(_) => url.path().to_string(),
    };
    if path.starts_with("//") {
        path = path.strip_prefix("/").unwrap().to_string();
    }
    // this isn't quite the format for gopher URLs as described by IETF (*),
    // but it seems to be used throughout gopherspace and undestood by clients.
    // (*) everybody will agree that IETF had a stupid idea
    // of using tab characters as separators in URL.
    Some(format!(
        "{}://{}:{}/{}{}",
        url.scheme(),
        url.host()?,
        url.port().unwrap_or(70),
        Into::<char>::into(t),
        path,
    ))
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
