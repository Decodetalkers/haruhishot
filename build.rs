fn main() {
    #[cfg(feature = "gui")]
    slint_build::compile("ui/selectwindow.slint").unwrap();
}
