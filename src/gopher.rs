use std::fmt::Display;
use std::str::FromStr;

use ansitok::{parse_ansi, parse_ansi_sgr, AnsiColor, ElementKind, VisualAttribute};
use anyhow::anyhow;
use async_std::stream::StreamExt;
use async_std::{
    io::{prelude::BufReadExt, BufReader, Cursor, ReadExt, WriteExt},
    net::TcpStream,
};

use serde::Deserialize;
use tide::{
    http::{mime, Mime},
    log,
};

const _INVALID_ENTRY: DirEntry = DirEntry {
    item_type: GopherItem::Unknown,
    label: String::new(),
    url: None,
};

const _ANSI_COLORS: &'static [&str] = &[
    "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
    "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
    "#000000", "#00005f", "#000087", "#0000af", "#0000d7", "#0000ff", "#005f00", "#005f5f",
    "#005f87", "#005faf", "#005fd7", "#005fff", "#008700", "#00875f", "#008787", "#0087af",
    "#0087d7", "#0087ff", "#00af00", "#00af5f", "#00af87", "#00afaf", "#00afd7", "#00afff",
    "#00d700", "#00d75f", "#00d787", "#00d7af", "#00d7d7", "#00d7ff", "#00ff00", "#00ff5f",
    "#00ff87", "#00ffaf", "#00ffd7", "#00ffff", "#5f0000", "#5f005f", "#5f0087", "#5f00af",
    "#5f00d7", "#5f00ff", "#5f5f00", "#5f5f5f", "#5f5f87", "#5f5faf", "#5f5fd7", "#5f5fff",
    "#5f8700", "#5f875f", "#5f8787", "#5f87af", "#5f87d7", "#5f87ff", "#5faf00", "#5faf5f",
    "#5faf87", "#5fafaf", "#5fafd7", "#5fafff", "#5fd700", "#5fd75f", "#5fd787", "#5fd7af",
    "#5fd7d7", "#5fd7ff", "#5fff00", "#5fff5f", "#5fff87", "#5fffaf", "#5fffd7", "#5fffff",
    "#870000", "#87005f", "#870087", "#8700af", "#8700d7", "#8700ff", "#875f00", "#875f5f",
    "#875f87", "#875faf", "#875fd7", "#875fff", "#878700", "#87875f", "#878787", "#8787af",
    "#8787d7", "#8787ff", "#87af00", "#87af5f", "#87af87", "#87afaf", "#87afd7", "#87afff",
    "#87d700", "#87d75f", "#87d787", "#87d7af", "#87d7d7", "#87d7ff", "#87ff00", "#87ff5f",
    "#87ff87", "#87ffaf", "#87ffd7", "#87ffff", "#af0000", "#af005f", "#af0087", "#af00af",
    "#af00d7", "#af00ff", "#af5f00", "#af5f5f", "#af5f87", "#af5faf", "#af5fd7", "#af5fff",
    "#af8700", "#af875f", "#af8787", "#af87af", "#af87d7", "#af87ff", "#afaf00", "#afaf5f",
    "#afaf87", "#afafaf", "#afafd7", "#afafff", "#afd700", "#afd75f", "#afd787", "#afd7af",
    "#afd7d7", "#afd7ff", "#afff00", "#afff5f", "#afff87", "#afffaf", "#afffd7", "#afffff",
    "#d70000", "#d7005f", "#d70087", "#d700af", "#d700d7", "#d700ff", "#d75f00", "#d75f5f",
    "#d75f87", "#d75faf", "#d75fd7", "#d75fff", "#d78700", "#d7875f", "#d78787", "#d787af",
    "#d787d7", "#d787ff", "#d7af00", "#d7af5f", "#d7af87", "#d7afaf", "#d7afd7", "#d7afff",
    "#d7d700", "#d7d75f", "#d7d787", "#d7d7af", "#d7d7d7", "#d7d7ff", "#d7ff00", "#d7ff5f",
    "#d7ff87", "#d7ffaf", "#d7ffd7", "#d7ffff", "#ff0000", "#ff005f", "#ff0087", "#ff00af",
    "#ff00d7", "#ff00ff", "#ff5f00", "#ff5f5f", "#ff5f87", "#ff5faf", "#ff5fd7", "#ff5fff",
    "#ff8700", "#ff875f", "#ff8787", "#ff87af", "#ff87d7", "#ff87ff", "#ffaf00", "#ffaf5f",
    "#ffaf87", "#ffafaf", "#ffafd7", "#ffafff", "#ffd700", "#ffd75f", "#ffd787", "#ffd7af",
    "#ffd7d7", "#ffd7ff", "#ffff00", "#ffff5f", "#ffff87", "#ffffaf", "#ffffd7", "#ffffff",
    "#080808", "#121212", "#1c1c1c", "#262626", "#303030", "#3a3a3a", "#444444", "#4e4e4e",
    "#585858", "#626262", "#6c6c6c", "#767676", "#808080", "#8a8a8a", "#949494", "#9e9e9e",
    "#a8a8a8", "#b2b2b2", "#bcbcbc", "#c6c6c6", "#d0d0d0", "#dadada", "#e4e4e4", "#eeeeee",
];

#[derive(PartialEq, Debug, Deserialize, Clone, Copy)]
pub enum GopherItem {
    TextFile,
    Submenu,
    Nameserver,
    Error,
    BinHex,
    Dos,
    UuencodeFile,
    FullTextSearch,
    Telnet,
    BinaryFile,
    Mirror,
    GifFile,
    ImageFile,
    Telnet3270,
    BitmapFile,
    MovieFile,
    SoundFile,
    DocFile,
    HtmlFile,
    Info,
    PngFile,
    RtfFile,
    WavFile,
    PdfFile,
    XmlFile,
    Unknown,
}

impl From<char> for GopherItem {
    fn from(c: char) -> GopherItem {
        match c {
            '0' => Self::TextFile,
            '1' => Self::Submenu,
            '2' => Self::Nameserver,
            '3' => Self::Error,
            '4' => Self::BinHex,
            '5' => Self::Dos,
            '6' => Self::UuencodeFile,
            '7' => Self::FullTextSearch,
            '8' => Self::Telnet,
            '9' => Self::BinaryFile,
            '+' => Self::Mirror,
            'g' => Self::GifFile,
            'I' => Self::ImageFile,
            'T' => Self::Telnet3270,
            ':' => Self::BitmapFile,
            ';' => Self::MovieFile,
            '<' => Self::SoundFile,
            'd' => Self::DocFile,
            'h' => Self::HtmlFile,
            'i' => Self::Info,
            'p' => Self::PngFile,
            'r' => Self::RtfFile,
            's' => Self::WavFile,
            'P' => Self::PdfFile,
            'X' => Self::XmlFile,
            _ => Self::Unknown,
        }
    }
}

impl Into<char> for GopherItem {
    fn into(self) -> char {
        match self {
            Self::TextFile => '0',
            Self::Submenu => '1',
            Self::Nameserver => '2',
            Self::Error => '3',
            Self::BinHex => '4',
            Self::Dos => '5',
            Self::UuencodeFile => '6',
            Self::FullTextSearch => '7',
            Self::Telnet => '8',
            Self::BinaryFile => '9',
            Self::Mirror => '+',
            Self::GifFile => 'g',
            Self::ImageFile => 'I',
            Self::Telnet3270 => 'T',
            Self::BitmapFile => ':',
            Self::MovieFile => ';',
            Self::SoundFile => '<',
            Self::DocFile => 'd',
            Self::HtmlFile => 'h',
            Self::Info => 'i',
            Self::PngFile => 'p',
            Self::RtfFile => 'r',
            Self::WavFile => 's',
            Self::PdfFile => 'P',
            Self::XmlFile => 'X',
            Self::Unknown => '?',
        }
    }
}

impl Into<Mime> for GopherItem {
    fn into(self) -> Mime {
        match self {
            Self::TextFile => mime::PLAIN,
            Self::Submenu => mime::HTML,
            Self::Nameserver => mime::PLAIN,
            Self::Error => mime::PLAIN,
            Self::BinHex => mime::BYTE_STREAM,
            Self::Dos => mime::BYTE_STREAM,
            Self::UuencodeFile => mime::PLAIN,
            Self::FullTextSearch => mime::HTML,
            Self::Telnet => mime::PLAIN,
            Self::BinaryFile => mime::BYTE_STREAM,
            Self::Mirror => mime::PLAIN,
            Self::GifFile => Mime::from_str("image/gif").unwrap_or(mime::BYTE_STREAM),
            Self::ImageFile => mime::JPEG,
            Self::Telnet3270 => mime::PLAIN,
            Self::BitmapFile => Mime::from_str("image/bmp").unwrap_or(mime::BYTE_STREAM),
            Self::MovieFile => mime::BYTE_STREAM,
            Self::SoundFile => mime::BYTE_STREAM,
            Self::DocFile => mime::BYTE_STREAM,
            Self::HtmlFile => mime::HTML,
            Self::Info => mime::PLAIN,
            Self::PngFile => mime::PNG,
            Self::RtfFile => mime::BYTE_STREAM,
            Self::WavFile => mime::BYTE_STREAM,
            Self::PdfFile => Mime::from_str("application/pdf").unwrap_or(mime::BYTE_STREAM),
            Self::XmlFile => mime::XML,
            Self::Unknown => mime::PLAIN,
        }
    }
}

impl Display for GopherItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<char>::into(self.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct GopherURL {
    pub host: String,
    pub port: u16,
    pub gopher_type: GopherItem,
    pub selector: String,
}

impl TryFrom<&str> for GopherURL {
    type Error = anyhow::Error;
    fn try_from(url_str: &str) -> Result<Self, Self::Error> {
        let gopher_url_re = regex_static::static_regex!(
            r#"(?:gopher://)?(?P<host>[^:/]+)(?::(?P<port>\d+))?(?:/(?P<type>[A-z0-9:+:;<?])(?P<selector>.*))?$"#
        );
        let Some(caps) = gopher_url_re.captures(url_str) else {
            return Err(anyhow!("failed to parse URL"));
        };
        log::info!("parsed {} as {:?}", url_str, caps);
        Ok(Self {
            host: String::from(caps.name("host").unwrap().as_str()),
            port: match caps.name("port") {
                Some(p) => p.as_str().parse().unwrap(),
                None => 70,
            },
            gopher_type: match caps.name("type") {
                Some(t) => t.as_str().chars().next().unwrap().into(),
                None => GopherItem::Submenu,
            },
            selector: match caps.name("selector") {
                Some(s) => String::from(s.as_str()),
                None => String::from(""),
            },
        })
    }
}

impl Display for GopherURL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.selector.is_empty() {
            write!(f, "gopher://{}:{}", self.host, self.port)
        } else {
            write!(
                f,
                "gopher://{}:{}/{}{}",
                self.host, self.port, self.gopher_type, self.selector
            )
        }
    }
}

impl GopherURL {
    fn new(host: &str, port: &str, item_type: &GopherItem, selector: &str) -> Self {
        Self {
            host: String::from(host),
            port: port.parse().unwrap_or(70),
            gopher_type: item_type.clone(),
            selector: String::from(selector),
        }
    }

    fn to_href(&self) -> Result<String, anyhow::Error> {
        if self.selector.starts_with("URL:") {
            Ok(String::from(&self.selector[4..]))
        } else {
            Ok(format!(
                "?url={}",
                urlencoding::encode(self.to_string().as_str())
            ))
        }
    }
}

#[derive(Debug)]
pub struct DirEntry {
    pub item_type: GopherItem,
    pub label: String,
    pub url: Option<GopherURL>,
}

impl From<&str> for DirEntry {
    fn from(value: &str) -> Self {
        let mut e = value.split('\t');
        match (e.next(), e.next(), e.next(), e.next()) {
            (Some(item_label), Some(selector), Some(host), Some(port)) => {
                let mut s = item_label.chars();
                let t: GopherItem = match s.next() {
                    Some(c) => c.into(),
                    None => {
                        return _INVALID_ENTRY;
                    }
                };
                let label: String = s.collect();
                DirEntry::new(t, label.as_str(), selector, host, port)
            }
            _ => _INVALID_ENTRY,
        }
    }
}

impl DirEntry {
    pub fn new(item_type: GopherItem, label: &str, selector: &str, host: &str, port: &str) -> Self {
        match item_type {
            GopherItem::Info => DirEntry {
                item_type,
                label: String::from(label),
                url: None,
            },
            _ => DirEntry {
                item_type,
                label: String::from(label),
                url: Some(GopherURL::new(host, port, &item_type, selector)),
            },
        }
    }

    pub fn to_href(&self) -> Option<String> {
        match &self.url {
            Some(url) => match url.to_href() {
                Ok(href) => Some(href),
                Err(e) => {
                    log::error!("invalid gopher URL: {:?}: {}", self.url, e);
                    None
                }
            },
            None => None,
        }
    }

    fn format_label(&self) -> String {
        match self.to_href() {
            Some(url) => format!(
                r#"<pre><a href="{}"">{}</a></pre>"#,
                url,
                &decode_ansi_style(&self.label)
            ),
            None => format!("<pre>{}</pre>", &decode_ansi_style(&self.label)),
        }
    }

    pub fn format_row(&self) -> Option<String> {
        match self.item_type {
            GopherItem::Unknown => None,
            GopherItem::Info => Some(format!("<td></td><td>{}</td>", self.format_label())),
            GopherItem::Submenu => Some(format!(
                "<td><i class=\"fa fa-folder-o\"></i></td><td>{}</td>",
                self.format_label()
            )),
            GopherItem::TextFile => Some(format!(
                "<td><i class=\"fa fa-file-text-o\"></i></td><td>{}</td>",
                self.format_label()
            )),
            GopherItem::HtmlFile => Some(format!(
                "<td><i class=\"fa fa-external-link\"></i></td><td>{}</td>",
                self.format_label()
            )),
            GopherItem::WavFile | GopherItem::SoundFile => Some(format!(
                r#"<td></td><td>
                    <pre>{0} (<a href="{1}">download</a>)</pre>
                    <audio controls><source src="{1}">Your browser does not support audio element.</audio>
                </td></tr>"#,
                html_escape::encode_text(&self.label),
                self.to_href().unwrap(),
            )),
            GopherItem::FullTextSearch => Some(format!(
                r#"<td><i class="fa fa-search"></i></td>
                    <td><form action="/" method="get">
                        <input name="query"  placeholder="{}" type="text">
                        <input type="hidden" name="url" value="{}">
                        <input type="hidden" name="t" value="{}">
                        <input type="submit" value="Submit">
                    </form></td><tr>"#,
                html_escape::encode_text(&self.label),
                self.url.as_ref().unwrap().to_string(),
                Into::<char>::into(self.item_type.clone()),
            )),
            GopherItem::ImageFile
            | GopherItem::BitmapFile
            | GopherItem::GifFile
            | GopherItem::PngFile => Some(format!(
                "<td></td><td><img src=\"{}\" />\n</tr>",
                self.to_href().unwrap()
            )),
            _ => Some(format!(
                "<td><i class=\"fa fa-file-o\"></i></td><td>{}</td>",
                self.format_label()
            )),
        }
    }
}

pub struct Menu {
    pub items: Vec<DirEntry>,
}

impl Menu {
    pub async fn from_url(url: &GopherURL, query: Option<String>) -> Result<Self, anyhow::Error> {
        let mut items: Vec<DirEntry> = Vec::new();
        let mut response = fetch_url(&url, query).await?.lines();
        while let Some(Ok(line)) = response.next().await {
            if line == "." {
                break;
            }
            let entry = DirEntry::from(line.as_str());
            match entry.item_type {
                GopherItem::Unknown => continue,
                GopherItem::Info => {
                    if let Some(item) = items.last_mut() {
                        // merge subsequent info items into one paragraph
                        // to preserve whatever pseudographic may be there
                        if item.item_type == GopherItem::Info {
                            item.label.push_str(format!("\n{}", entry.label).as_str());
                            continue;
                        }
                    }
                    items.push(entry)
                }
                _ => items.push(entry),
            }
        }

        Ok(Self { items: items })
    }
}

pub async fn fetch_url(
    url: &GopherURL,
    query: Option<String>,
) -> Result<impl BufReadExt, anyhow::Error> {
    let mut stream = TcpStream::connect(format!("{}:{}", url.host, url.port,)).await?;
    let selector = match urlencoding::decode(
        match query {
            Some(q) => format!("{}\t{}\r\n", url.selector, q),
            None => format!("{}\r\n", url.selector),
        }
        .as_str(),
    ) {
        Ok(s) => s.into_owned(),
        Err(e) => {
            return Err(anyhow!("decoding URL: {}", e));
        }
    };
    stream
        .write_all(urlencoding::decode(&selector).unwrap().as_bytes())
        .await?;
    let mut buf = BufReader::new(stream);

    /*
       Since gopher has no way to specify any metadata in its response,
       so instead of actual content there may be a dir entry with error.
       To handle this, we peek into response to see if it is
       possible to parse it into dir entry and whether there is an error.
       If not, returns original content.
    */
    let mut header = vec![0; 256];
    let bytes_read = buf.read(&mut header).await?;
    if let Ok(first_line) = String::from_utf8(header.clone()) {
        match DirEntry::from(first_line.as_str()) {
            entry if entry.item_type == GopherItem::Error => {
                log::error!("got error fetching {}: {}", url, entry.label);
                return Err(anyhow!(entry.label));
            }
            _ => {}
        }
    }
    Ok(Cursor::new(header[0..bytes_read].to_vec()).chain(buf))
}

fn decode_ansi_style(text: &str) -> String {
    let mut result = String::new();
    let mut span_style: Vec<String> = Vec::new();
    for token in parse_ansi(text) {
        let txt = &text[token.start()..token.end()];
        match token.kind() {
            ElementKind::Text => {
                if !span_style.is_empty() {
                    result.push_str(&format!(
                        r#"<span style="{}">{}</span>"#,
                        span_style.join(";"),
                        html_escape::encode_text(txt),
                    ))
                } else {
                    result.push_str(txt)
                }
            }
            ElementKind::Sgr => {
                for style in parse_ansi_sgr(txt) {
                    match style.as_escape() {
                        // TODO: more styles?
                        Some(VisualAttribute::FgColor(c)) => {
                            span_style.push(format!("color:{}", to_color(c)))
                        }
                        Some(VisualAttribute::BgColor(c)) => {
                            span_style.push(format!("color:{}", to_color(c)))
                        }
                        Some(VisualAttribute::Reset(_)) => span_style.clear(),
                        Some(_) => continue,
                        None => continue,
                    }
                }
            }
            _ => {}
        }
    }
    return result;
}

fn to_color(c: AnsiColor) -> String {
    match c {
        AnsiColor::Bit4(v) | AnsiColor::Bit8(v) => String::from(_ANSI_COLORS[usize::from(v)]),
        AnsiColor::Bit24 { r, g, b } => format!("rgb({r}, {g}, {b}"),
    }
}

#[cfg(test)]
mod tests {
    use ansitok::{parse_ansi, parse_ansi_sgr, ElementKind, Output};

    use super::*;

    #[test]
    fn parsing_entries() {
        let mut e = DirEntry::from("1Test entry\t/test\texample.com\t70\r\n");
        assert_eq!(e.label, "Test entry");
        assert_eq!(e.item_type, GopherItem::Submenu);
        assert_eq!(e.url.unwrap().host, "example.com");
        e = DirEntry::from("0test2	selector	1.1.1.1	70\r\n");
        assert_eq!(e.label, "test2");
        assert_eq!(e.item_type, GopherItem::TextFile);
        let url = e.url.unwrap();
        assert_eq!(url.host, "1.1.1.1");
        assert_eq!(url.selector, "selector");
        assert_eq!(url.gopher_type, GopherItem::TextFile);
    }

    #[test]
    fn parsing_urls() {
        let mut u = GopherURL::try_from("gopher://example.com/0/path/to/document").unwrap();
        assert_eq!(u.gopher_type, GopherItem::TextFile);
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 70);
        assert_eq!(u.selector, "/path/to/document");
        assert_eq!(u.to_string(), "gopher://example.com:70/0/path/to/document");

        u = GopherURL::try_from("gopher://example2.com:71").unwrap();
        assert_eq!(u.gopher_type, GopherItem::Submenu);
        assert_eq!(u.host, "example2.com");
        assert_eq!(u.port, 71);
        assert_eq!(u.selector, "");
        assert_eq!(u.to_string(), "gopher://example2.com:71");

        u = GopherURL::try_from("gopher://khzae.net:70/</music/khzae/khzae.ogg").unwrap();
        assert_eq!(u.gopher_type, GopherItem::SoundFile);
        assert_eq!(u.host, "khzae.net");
        assert_eq!(u.port, 70);

        u = GopherURL::new("1.1.1.1", "70", &GopherItem::TextFile, "some-selector");
        assert_eq!(u.to_string(), "gopher://1.1.1.1:70/0some-selector");
    }

    #[test]
    fn ansi_codes() {
        let text = "[38;5;250mW[0m[38;5;143ma[0m[38;5;145mr[0m[38;5;250me[0m[38;5;250mz[0m";
        for token in parse_ansi(text) {
            match token.kind() {
                ElementKind::Sgr => {
                    let sgr = &text[token.start()..token.end()];
                    for style in parse_ansi_sgr(sgr) {
                        println!("style={:?}", style);
                        let style = style.as_escape().unwrap();
                        println!("style={:?}", style);
                    }
                }
                ElementKind::Text => println!("{}", &text[token.start()..token.end()]),
                _ => (),
            }
        }
    }
}
