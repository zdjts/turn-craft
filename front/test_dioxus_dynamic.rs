use dioxus::prelude::*;
#[derive(Props, Clone, PartialEq)]
struct MyProps { a: i32 }
fn CompA(props: MyProps) -> Element { rsx! { div { "{props.a}" } } }
fn main() {
    let comp = CompA;
    let _ = rsx! {
        comp { a: 1 }
    };
}
