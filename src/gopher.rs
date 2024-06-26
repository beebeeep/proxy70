use std::fmt::Display;
use std::str::FromStr;

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
                html_escape::encode_text(&self.label)
            ),
            None => format!("<pre>{}</pre>", html_escape::encode_text(&self.label)),
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

#[cfg(test)]
mod tests {
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
}
