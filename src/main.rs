use std::io::{BufRead, Read};
use std::path::Path;

use anyhow::Error;

/// ```
/// <!-- MD_PACKER_INLINE BEGIN -->
/// ![stuff](http://example.com/foo.jpg)
/// <!-- MD_PACKER_INLINE END -->
/// ```
/// becomes
/// ```
/// ![stuff](data:image/png;base64,...")
/// ```

fn main() -> Result<(), Error> {
    let regex = regex::Regex::new("!\\[([^\\]]*)\\]\\(([^)]*)\\)")?;
    let pwd = std::env::current_dir()?;
    let mut inline = false;
    for line in std::io::BufReader::new(std::io::stdin()).lines() {
        let line = line?;
        if line == "<!-- MD_PACKER_INLINE BEGIN -->" {
            inline = true;
            continue;
        }
        if line == "<!-- MD_PACKER_INLINE END -->" {
            inline = false;
            continue;
        }
        if inline {
            println!(
                "{}",
                regex.replace(&line, |caps: &regex::Captures| {
                    match replacer(pwd.as_ref(), caps) {
                        Ok(a) => a,
                        Err(e) => {
                            eprintln!("Error fetching image: {}", e);
                            format!("{}", &caps[0])
                        }
                    }
                })
            );
        } else {
            println!("{}", line);
        }
    }
    Ok(())
}

fn replacer(pwd: &Path, caps: &regex::Captures) -> Result<String, Error> {
    let (content_type, data): (String, bytes::Bytes) =
        if let Ok::<reqwest::Url, _>(url) = caps[2].parse() {
            let image_req = reqwest::blocking::get(url)?;
            let content_type = image_req
                .headers()
                .get("content-type")
                .ok_or_else(|| anyhow::anyhow!("missing content-type"))?;
            (content_type.to_str()?.to_string(), image_req.bytes()?)
        } else {
            let path = Path::new(&caps[2]);
            let mut data = Vec::with_capacity(path.metadata()?.len() as usize);
            std::fs::File::open(pwd.join(path))?.read_to_end(&mut data)?;
            (
                format!(
                    "image/{}",
                    path.extension()
                        .ok_or_else(|| anyhow::anyhow!("unknown image type"))?
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("unknown image type"))?
                ),
                data.into(),
            )
        };
    Ok(format!(
        "![{}](data:{};base64,{})",
        &caps[1],
        content_type,
        base64::display::Base64Display::with_config(
            &data,
            base64::Config::new(base64::CharacterSet::UrlSafe, true)
        )
    ))
}
