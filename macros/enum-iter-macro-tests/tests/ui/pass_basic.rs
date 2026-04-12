use enum_iter_macro::EnumIter;

#[derive(EnumIter)]
enum Foo {
    A,
    B,
    C,
}

fn main() {
    assert_eq!(Foo::iter().count(), 3);
}
