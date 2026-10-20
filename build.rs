//! Optional Windows icon embed when assets/icon.ico is present.

fn main() {
	#[cfg(windows)]
	{
		if std::path::Path::new("assets/icon.ico").is_file() {
			let mut res = winresource::WindowsResource::new();
			res.set_icon("assets/icon.ico");
			if let Err(e) = res.compile() {
				eprintln!("cargo:warning=winresource icon embed failed: {e}");
			}
		}
	}
}
