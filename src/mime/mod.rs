use std::{
    fmt::{
        self,
        Display,
        Debug
    },
    str::FromStr,
    io::Error
};
use Mime::*;

#[derive(PartialEq, Eq)]
pub enum Mime {
    Gif,
    Jpeg,
    Png,
    Svg,
    Webp,
    Css,
    JavaScript,
    Json,
    Plain,
    Html
}

impl<T: AsRef<str>> From<T> for Mime {
    fn from(from: T) -> Self {
        match from.as_ref() {
            "gif" => Gif, 
            "jpeg" => Jpeg, 
            "png" => Png, 
            "svg" => Svg, 
            "webp" => Webp, 
            "css" => Css, 
            "js" => JavaScript,
            "json" => Json,
            "html" => Html, 
            _ => Plain
        }
    }
}

impl FromStr for Mime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image/gif" => Ok(Gif), 
            "image/jpeg" => Ok(Jpeg), 
            "image/png" => Ok(Png), 
            "image/svg+xml" => Ok(Svg), 
            "image/webp" => Ok(Webp), 
            "text/css" => Ok(Css), 
            "text/js" => Ok(JavaScript),
            "application/json" => Ok(Json),
            "text/html" => Ok(Html), 
            _ => Ok(Plain)
        }
    }
}

impl Display for Mime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gif | Jpeg | Png | Webp => write!(f, "image/{self:?}")?,
            Svg => write!(f, "image/svg+xml")?,
            Css | JavaScript | Plain | Html => write!(f, "text/{self:?}")?,
            Json => write!(f, "application/{self:?}")?,
        }

        Ok(())
    }
}

impl Debug for Mime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gif => write!(f, "gif")?,
            Jpeg => write!(f, "jpeg")?,
            Png => write!(f, "png")?,
            Webp => write!(f, "webp")?,
            Svg => write!(f, "svg")?,
            Css => write!(f, "css")?,
            JavaScript => write!(f, "javascript")?,
            Json => write!(f, "json")?,
            Plain => write!(f, "plain")?,
            Html => write!(f, "html")?
        }

        Ok(())
    }
}

impl Default for Mime {
    fn default() -> Self {
        Plain
    }
}