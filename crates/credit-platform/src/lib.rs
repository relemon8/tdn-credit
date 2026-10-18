use std::{io, path::PathBuf};

pub fn data_dir(portable: bool) -> io::Result<PathBuf> {
    if portable {
        return Ok(std::env::current_dir()?.join("data"));
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("TDN CREDIT"))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA unavailable"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) =
            std::env::var_os("XDG_DATA_HOME").filter(|p| PathBuf::from(p).is_absolute())
        {
            return Ok(PathBuf::from(path).join("tdn-credit"));
        }
        let home = std::env::var_os("HOME").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "HOME unavailable; use --portable")
        })?;
        #[cfg(target_os = "macos")]
        let relative = "Library/Application Support/TDN CREDIT";
        #[cfg(not(target_os = "macos"))]
        let relative = ".local/share/tdn-credit";
        Ok(PathBuf::from(home).join(relative))
    }
}
pub fn open_browser(url: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(url).spawn()?.wait()?;
    Ok(())
}
