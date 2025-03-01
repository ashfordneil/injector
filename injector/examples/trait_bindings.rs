use injector::{Injectable, Injector, binding, multi_binding};

fn main() {
    let injector = Injector::new();
    let trait_object = injector.get_trait_object::<dyn SayHello>();
    println!("{}", trait_object.say_hello());

    let set_of_trait_objects = injector.get_all_trait_objects::<dyn WeAllSayHello>();
    let mut output = String::new();
    for trait_object in set_of_trait_objects {
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
