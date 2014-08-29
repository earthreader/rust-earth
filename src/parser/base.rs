use std::cell::RefCell;
use std::collections::hashmap::HashMap;

use xml::common::{Name, Attribute};
use xml::namespace::Namespace;
use xml::reader::events::{XmlEvent, StartElement, EndElement, Characters};

pub struct Element {
    pub name: Name,
    pub attributes: Vec<Attribute>,
    pub namespace: Namespace,
    pub text: String,
}

impl Element {
    fn new(name: Name, attributes: Vec<Attribute>, namespace: Namespace) -> Element {
        Element {
            name: name,
            attributes: attributes,
            namespace: namespace,
            text: "".into_string(),
        }
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<&'a str> {
        self.attributes.iter()
            .find(|&attr| attr.name.local_name.as_slice() == key)
            .map(|e| e.value.as_slice())
    }
}

// pub type ParseResult<T> = Result<T, ParseError>;

// pub type ParseError = ();

// pub trait ParserHandler<T, R> {
//     fn on_start(&Element, T) -> (T, Self);
//     fn on_emit(&mut self, &XmlEvent);
//     fn on_end(&self, &Element, T) -> R;
// }

type NameKey = (Option<String>, String);
type ParserMap<T, R> = HashMap<NameKey, Parser<T, R>>;
type ChildrenMap<R> = HashMap<String, Vec<R>>;
type OnStartHandler<T> = |&Element, &mut T|: 'static;
type ParserFn<T, R> = fn(&Element, ChildrenMap<R>, T) -> R;

pub struct Parser<T, R> {
    attr_name: String,
    on_start: Option<RefCell<OnStartHandler<T>>>,
    func: ParserFn<T, R>,
    children: ParserMap<T, R>,
}

pub enum ParserBuilder<T, R> {
    Root(ParserMap<T, R>),
    Children {
        parent: Box<ParserBuilder<T, R>>,
        name_key: NameKey,
        parser: Parser<T, R>,
    }
}

impl<T, R> ParserBuilder<T, R> {
    pub fn new() -> ParserBuilder<T, R> {
        Root(HashMap::new())
    }

    pub fn path<'a>(self, element_name: &str, func: ParserFn<T, R>) -> ParserBuilder<T, R> {
        let child = Children {
            parent: box self,
            name_key: (None, element_name.to_string()),
            parser: Parser {
                attr_name: element_name.to_string(),
                on_start: None,
                func: func,
                children: HashMap::new(),
            },
        };
        child
    }

    pub fn on_start(mut self, handler: OnStartHandler<T>) -> ParserBuilder<T, R> {
        match self {
            Children { ref mut parser, .. } => {
                parser.on_start = Some(RefCell::new(handler));
            }
            _ => { fail!() }
        }
        self
    }

    pub fn attr_name(mut self, name: String) -> ParserBuilder<T, R> {
        match self {
            Children { ref mut parser, .. } => {
                parser.attr_name = name;
            }
            _ => { fail!() }
        }
        self
    }

    pub fn end(self) -> ParserBuilder<T, R> {
        match self {
            Root(_) => { fail!() }
            Children { parent, name_key, parser } => {
                let mut parent = *parent;
                parent.insert_parser(name_key, parser);
                parent
            }
        }
    }

    pub fn build(self) -> ParserBase<T, R> {
        match self {
            Root(p) => { ParserBase { root: p } }
            _ => { fail!() }
        }
    }

    fn insert_parser(&mut self, key: NameKey, value: Parser<T, R>) {
        let parsers = match *self {
            Root(ref mut parsers) => parsers,
            Children { ref mut parser, .. } => &mut parser.children,
        };
        parsers.insert(key, value);
    }
}

enum ParsingState<'a, T, R> {
    Skip,
    State {
        parser: &'a Parser<T, R>,
        prev_parsers: &'a ParserMap<T, R>,
        element: Element,
        children: ChildrenMap<R>,
        prev_session: T,
    },
}

pub struct ParserBase<T, R> {
    root: ParserMap<T, R>,
}

impl<T: Clone, R> ParserBase<T, R> {
    pub fn parse<I: Iterator<XmlEvent>>(&self, reader: &mut I, mut session: T) -> R {
        let mut stack: Vec<ParsingState<_, _>> = Vec::new();
        let mut parsers = &self.root;
        for event in *reader {
            match event {
                StartElement { name, attributes, namespace } => {
                    if stack.last().map_or(false, |s| match *s { Skip => true, _ => false }) {
                        stack.push(Skip);
                        continue;
                    }
                    let key = (name.namespace.clone(), name.local_name.clone());
                    match parsers.find(&key) {
                        None => { stack.push(Skip); continue; }
                        Some(p) => {
                            let element = Element::new(name, attributes, namespace);
                            let prev_session = session.clone();
                            parsers = &p.children;
                            match p.on_start {
                                None => { }
                                Some(ref c) => { (*c.borrow_mut().deref_mut())(&element, &mut session) }
                            }
                            stack.push(State {
                                parser: p,
                                prev_parsers: parsers,
                                element: element,
                                children: HashMap::new(),
                                prev_session: prev_session,
                            });
                        }
                    }
                },
                Characters(text) => {
                    match stack.mut_last() {
                        None => { continue }
                        Some(&Skip) => { continue }
                        Some(&State { ref mut element, .. }) => {
                            element.text.push_str(text.as_slice());
                        }
                    }
                }
                EndElement { .. } => {
                    let (parser, value) = match stack.pop() {
                        None => { fail!() }
                        Some(Skip) => { continue }
                        Some(State{ parser, prev_parsers, element, children, prev_session }) => {
                            let v = (parser.func)(&element, children, session);
                            parsers = prev_parsers;
                            session = prev_session;
                            (parser, v)
                        }
                    };
                    match stack.mut_last() {
                        None => { return value; }
                        Some(&Skip) => { fail!("??? it's impossible"); }
                        Some(&State { ref mut children, .. }) => {
                            children.find_with_or_insert_with(
                                parser.attr_name.clone(), value,
                                |_key, already, new| already.push(new),
                                |_key, value| vec![value],
                            );
                        }
                    }
                }
                _ => { }
            }
        }
        fail!();
    }

    // fn traverse<I: Iterator<XmlEvent>>(&self, reader: &mut I, root: &mut feed::Element, session: T) {
    //     let mut skipped_tree_depth = 0i;
    //     loop {
    //         let event = match reader.next() {
    //             Some(e) => e,
    //             None => break
    //         };
    //         match event {
    //             StartElement { name, attributes, namespace } => {
    //                 if skipped_tree_depth == 0 {
    //                     let key = (name.namespace.clone(), name.local_name.clone());
    //                     match self.children.find(&key) {
    //                         None => { skipped_tree_depth += 1; continue; }
    //                         Some(&(ref parser, ref attr_name)) => {
    //                             let session = session.clone();
    //                             let (mut child, session) = (parser.parser)(&name, attributes.as_slice(), &namespace, session);
    //                             let value = match child {
    //                                 None => { skipped_tree_depth += 1; continue; }
    //                                 Some(Elem(mut e)) => { parser.traverse(reader, &mut e, session); Elem(e) }
    //                                 Some(v) => { skipped_tree_depth += 1; v }
    //                             };
    //                             root.fields.insert(attr_name.to_string(), value);
    //                         }
    //                     }
    //                 }
    //             }
    //             EndElement { .. } => {
    //                 skipped_tree_depth -= 1;
    //                 if skipped_tree_depth < 0 {
    //                     break
    //                 }
    //             }
    //             _ => {}
    //         }
    //     }
    // }
}
