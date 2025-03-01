use injector::{Injectable, Injector, binding, multi_binding};

fn main() {
    let injector = Injector::new();
    let everything: &Everything = injector.get();
    println!("{}", everything.say_hello.say_hello());

    let mut output = String::new();
    for trait_object in &everything.we_all_say_hello {
        trait_object.also_say_hello(&mut output);
    }
    println!("{}", output);
}

trait SayHello {
    fn say_hello(&self) -> String;
}

trait WeAllSayHello {
    fn also_say_hello(&self, say_hello_into: &mut String);
}

#[derive(Injectable)]
struct SayHelloImpl;

#[derive(Injectable)]
struct SecondSayHelloImpl;

#[binding]
impl SayHello for SayHelloImpl {
    fn say_hello(&self) -> String {
        "Hello from the concrete impl".to_string()
    }
}

#[multi_binding]
impl WeAllSayHello for SayHelloImpl {
    fn also_say_hello(&self, say_hello_into: &mut String) {
        if !say_hello_into.is_empty() {
            say_hello_into.push('\n');
        }
        say_hello_into.push_str("We all say hello, from the concrete impl");
    }
}

#[multi_binding]
impl WeAllSayHello for SecondSayHelloImpl {
    fn also_say_hello(&self, say_hello_into: &mut String) {
        if !say_hello_into.is_empty() {
            say_hello_into.push('\n');
        }
        say_hello_into.push_str("We all say hello, from the second concrete impl");
    }
}

#[derive(Injectable)]
struct Everything<'a> {
    say_hello: &'a dyn SayHello,
    #[from_multi_binding(dyn WeAllSayHello)]
    we_all_say_hello: Vec<&'a dyn WeAllSayHello>,
}
