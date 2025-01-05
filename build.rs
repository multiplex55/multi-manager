extern crate winres;

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("Running build.rs...");
        //App icon
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/app_icon.ico"); // Path to your .ico file
        res.compile().expect("Failed to compile resources");
        println!("Finished running build.rs");
    }
}
