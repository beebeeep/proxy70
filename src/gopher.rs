use async_std::{
    io::{BufReader, ReadExt, WriteExt},
    net::TcpStream,
};
use tide::log;
use url::Url;

#[derive(PartialEq, Debug)]
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

#[derive(Debug)]
pub struct DirEntry {
    pub item_type: GopherItem,
    pub label: String,
    pub url: Option<Url>,
}

impl From<&str> for DirEntry {
    fn from(value: &str) -> Self {
        let mut e = value.split('\t');
        match (e.next(), e.next(), e.next(), e.next()) {
            (Some(item_label), Some(selector), Some(host), Some(port)) => {
                let mut s = item_label.chars();
                let t: GopherItem = s.next().unwrap().into();
                let label: String = s.collect();
                DirEntry::new(t, label.as_str(), selector, host, port)
            }
            _ => DirEntry {
                item_type: GopherItem::Unknown,
                label: String::from("[invalid entry]"),
                url: None,
            },
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
                url: get_url(selector, host, port),
            },
        }
    }
}

pub async fn fetch_url(url: Url) -> Result<BufReader<TcpStream>, anyhow::Error> {
    let mut stream = TcpStream::connect(format!(
        "{}:{}",
        url.host().unwrap(),
        url.port().unwrap_or(70),
    ))
    .await?;
    // let mut result = Vec::new();
    stream
        .write_all(format!("{}\r\n", url.path()).as_bytes())
        .await?;
    let mut buf = BufReader::new(stream);
    Ok(buf)
}

pub async fn fetch_directory(url: Url) -> Result<Vec<DirEntry>, anyhow::Error> {
    todo!("implement");
}

fn get_url(selector: &str, host: &str, port: &str) -> Option<Url> {
    let url_str: String;
    if selector.starts_with("URL:") {
        url_str = String::from(&selector[4..])
    } else {
        url_str = format!("gopher://{}:{}{}", host, port, selector)
    };

    match Url::parse(&url_str) {
        Ok(url) => Some(url),
        Err(e) => {
            log::error!("parsing url: {:}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_entries() {
        let e = DirEntry::from("1Test entry\t/test\texample.com\t70\r\n");
        assert_eq!(e.label, "Test entry");
        assert_eq!(e.item_type, GopherItem::Submenu);
        assert_eq!(
            e.url,
            Some(Url::parse("gopher://example.com:70/test").unwrap())
        );
    }
}
