use async_std::{
    io::{ReadExt, WriteExt},
    net::TcpStream,
};
use url::Url;

#[derive(PartialEq, Debug)]
enum GopherItem {
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

struct DirEntry {
    item_type: GopherItem,
    label: String,
    url: Url,
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

impl From<&str> for DirEntry {
    fn from(value: &str) -> Self {
        let mut e = value.split('\t');
        match (e.next(), e.next(), e.next(), e.next()) {
            (Some(item_label), Some(selector), Some(host), Some(port)) => {
                let url =
                    Url::parse(format!("gopher://{}:{}{}", host, port, selector).as_str()).unwrap();
                let mut s = item_label.chars();
                let t: GopherItem = s.next().unwrap().into();
                let label: String = s.collect();
                DirEntry {
                    item_type: t,
                    label,
                    url,
                }
            }
            _ => DirEntry {
                item_type: GopherItem::Unknown,
                label: String::from("<invalid entry>"),
                url: Url::parse("gopher://error.example.com:1").unwrap(),
            },
        }
    }
}

pub async fn fetch_url(url: Url) -> Result<Vec<u8>, anyhow::Error> {
    let mut stream = TcpStream::connect(format!(
        "{}:{}",
        url.host().unwrap(),
        url.port().unwrap_or(70),
    ))
    .await?;
    let mut result = Vec::new();
    stream
        .write_all(format!("{}\r\n", url.path()).as_bytes())
        .await?;
    stream.read_to_end(&mut result).await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_entries() {
        let e = DirEntry::from("1Test entry\t/test\texample.com\t70\r\n");
        assert_eq!(e.label, "Test entry");
        assert_eq!(e.item_type, GopherItem::Submenu);
        assert_eq!(e.url, Url::parse("gopher://example.com:70/test").unwrap());
    }
}
