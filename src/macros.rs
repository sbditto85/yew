//! This module contains macros which implement the `html!` macro and JSX-like templates.

use std::borrow::Cow;
use html::Component;
use virtual_dom::{Listener, VNode};

#[doc(hidden)]
#[macro_export]
macro_rules! local_stringify {
    ($s:ident) => {
        stringify!($s)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! local_vec {
    [$($e:expr),+] => {
        vec![$($e,)+]
    }
}

/// The html! entrypoint and this implementation had to be separated to prevent infinite recursion.
#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! html_impl {
    ($stack:ident (< > $($tail:tt)*)) => {
        let vlist = $crate::virtual_dom::VList::new();
        $stack.push(vlist.into());
        html_impl! { $stack ($($tail)*) }
    };
    ($stack:ident (< / > $($tail:tt)*)) => {
        $crate::macros::child_to_parent(&mut $stack, None);
        html_impl! { $stack ($($tail)*) }
    };
    // Begin component tag
    // eg: <$type: ..
    ($stack:ident (< $comp:ty : $($tail:tt)*)) => {
        #[allow(unused_mut)]
        let mut pair = $crate::virtual_dom::VComp::lazy::<$comp>();
        html_impl! { @vcomp $stack pair ($($tail)*) }
    };
    // Set a whole struct as properties
    // eg: with $ident, ..
    (@vcomp $stack:ident $pair:ident (with $props:ident, $($tail:tt)*)) => {
        $pair.0 = $props;
        html_impl! { @vcomp $stack $pair ($($tail)*) }
    };
    // Set a specific field of the properties
    // It uses `Transformer` trait to convert a type used in template to a type of the field.
    // eg: $ident = $expr, ..
    (@vcomp $stack:ident $pair:ident ($attr:ident = $val:expr, $($tail:tt)*)) => {
        // It cloned for ergonomics in templates. Attribute with
        // `self.param` value could be reused and sholdn't be cloned
        // by yourself
        ($pair.0).$attr = $crate::virtual_dom::vcomp::Transformer::transform(&mut $pair.1, $val);
        html_impl! { @vcomp $stack $pair ($($tail)*) }
    };
    // Tag self-close
    // eg: /> ..
    (@vcomp $stack:ident $pair:ident (/ > $($tail:tt)*)) => {
        let (props, mut comp) = $pair;
        comp.set_props(props);
        $stack.push(comp.into());
        $crate::macros::child_to_parent(&mut $stack, None);
        html_impl! { $stack ($($tail)*) }
    };
    // Begin open svg tag
    // Sets the xmlns to the svg namespace
    // eg: <svg ..
    ($stack:ident (< svg $($tail:tt)*)) => {
        let vtag = $crate::virtual_dom::VTag::new("svg", Some("http://www.w3.org/2000/svg".into()));
        $stack.push(vtag.into());
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Begin open tag
    // eg: <$ident ..
    ($stack:ident (< $starttag:ident $($tail:tt)*)) => {
        let vtag = $crate::virtual_dom::VTag::new(local_stringify!($starttag), None);
        $stack.push(vtag.into());
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Set namespace
    (@vtag $stack:ident (xmlns = $value:expr, $($tail:tt)*)) => {
        $crate::macros::set_namespace(&mut $stack, Some($value.into()));
        html_impl! { @vtag $stack ($($tail)*) }
    };

    // Set multiple classes
    // PATTERN: class=("class-1", "class-2", local_variable),
    // eg: class=($expr, ...), ..
    (@vtag $stack:ident (class = ($($class:expr),*), $($tail:tt)*)) => {
        $( $crate::macros::append_class(&mut $stack, $class); )*
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Set a single class
    // eg: class=$expr, ..
    (@vtag $stack:ident (class = $class:expr, $($tail:tt)*)) => {
        $crate::macros::set_classes(&mut $stack, $class);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Set value
    // PATTERN: value="",
    // eg: value=$expr, ..
    (@vtag $stack:ident (value = $value:expr, $($tail:tt)*)) => {
        $crate::macros::set_value_or_attribute(&mut $stack, $value);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // PATTERN: attribute=value, - workaround for `type` attribute
    // because `type` is a keyword in Rust
    // eg: type=$expr, ..
    (@vtag $stack:ident (type = $kind:expr, $($tail:tt)*)) => {
        $crate::macros::set_kind(&mut $stack, $kind);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // eg: checked=$expr, ..
    (@vtag $stack:ident (checked = $kind:expr, $($tail:tt)*)) => {
        $crate::macros::set_checked(&mut $stack, $kind);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // eg: disabled=$expr, ..
    (@vtag $stack:ident (disabled = $kind:expr, $($tail:tt)*)) => {
        if $kind {
            $crate::macros::add_attribute(&mut $stack, "disabled", "true");
        }
        html_impl! { @vtag $stack ($($tail)*) }
    };
    (@vtag $stack:ident (selected = $kind:expr, $($tail:tt)*)) => {
        if $kind {
            $crate::macros::add_attribute(&mut $stack, "selected", "selected");
        }
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Events:
    // TODO Try to rewrite with procedural macro.
    (@vtag $stack:ident (onclick = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onclick) = move | $var: $crate::events::ClickEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondoubleclick = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondoubleclick) = move | $var: $crate::events::DoubleClickEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onkeypress = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onkeypress) = move | $var: $crate::events::KeyPressEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onkeydown = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onkeydown) = move | $var: $crate::events::KeyDownEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onkeyup = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onkeyup) = move | $var: $crate::events::KeyUpEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmousedown = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmousedown) = move | $var: $crate::events::MouseDownEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmousemove = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmousemove) = move | $var: $crate::events::MouseMoveEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmouseout = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmouseout) = move | $var: $crate::events::MouseOutEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmouseenter = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmouseenter) = move | $var: $crate::events::MouseEnterEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmouseleave = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmouseleave) = move | $var: $crate::events::MouseLeaveEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmousewheel = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmousewheel) = move | $var: $crate::events::MouseWheelEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmouseover = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmouseover) = move | $var: $crate::events::MouseOverEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onmouseup = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onmouseup) = move | $var: $crate::events::MouseUpEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ongotpointercapture = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ongotpointercapture) = move | $var: $crate::events::GotPointerCaptureEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onlostpointercapture = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onlostpointercapture) = move | $var: $crate::events::LostPointerCaptureEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointercancel = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointercancel) = move | $var: $crate::events::PointerCancelEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerdown = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerdown) = move | $var: $crate::events::PointerDownEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerenter = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerenter) = move | $var: $crate::events::PointerEnterEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerleave = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerleave) = move | $var: $crate::events::PointerLeaveEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointermove = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointermove) = move | $var: $crate::events::PointerMoveEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerout = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerout) = move | $var: $crate::events::PointerOutEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerover = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerover) = move | $var: $crate::events::PointerOverEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onpointerup = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onpointerup) = move | $var: $crate::events::PointerUpEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onscroll = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onscroll) = move | $var: $crate::events::ScrollEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onblur = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onblur) = move | $var: $crate::events::BlurEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onfocus = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onfocus) = move | $var: $crate::events::FocusEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onsubmit = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onsubmit) = move | $var: $crate::events::SubmitEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (oninput = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((oninput) = move | $var: $crate::events::InputData | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (onchange = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((onchange) = move | $var: $crate::events::ChangeData | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragstart = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragstart) = move | $var: $crate::events::DragStartEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondrag = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondrag) = move | $var: $crate::events::DragEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragend = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragend) = move | $var: $crate::events::DragEndEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragenter = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragenter) = move | $var: $crate::events::DragEnterEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragleave = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragleave) = move | $var: $crate::events::DragLeaveEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragover = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragover) = move | $var: $crate::events::DragOverEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondragexit = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondragexit) = move | $var: $crate::events::DragExitEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (ondrop = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((ondrop) = move | $var: $crate::events::DragDropEvent | $handler, $($tail)*) }
    };
    (@vtag $stack:ident (oncontextmenu = | $var:pat | $handler:expr, $($tail:tt)*)) => {
        html_impl! { @vtag $stack ((oncontextmenu) = move | $var: $crate::events::ContextMenuEvent | $handler, $($tail)*) }
    };

    // PATTERN: (action)=expression,
    // eg: ($ident)=$expr, ..
    (@vtag $stack:ident (($action:ident) = $handler:expr, $($tail:tt)*)) => {
        // Catch value to a separate variable for clear error messages
        let handler = $handler;
        let listener = $crate::html::$action::Wrapper::from(handler);
        $crate::macros::attach_listener(&mut $stack, Box::new(listener));
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Attributes
    // eg: href=$expr, ..
    (@vtag $stack:ident (href = $href:expr, $($tail:tt)*)) => {
        let href: $crate::html::Href = $href.into();
        $crate::macros::add_attribute(&mut $stack, "href", href);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // eg: $ident=$expr, ..
    (@vtag $stack:ident ($attr:ident = $val:expr, $($tail:tt)*)) => {
        $crate::macros::add_attribute(&mut $stack, local_stringify!($attr), $val);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Terminate open tag
    // eg: > ..
    (@vtag $stack:ident (> $($tail:tt)*)) => {
        html_impl! { $stack ($($tail)*) }
    };
    // Self-close tag
    // eg: /> ..
    (@vtag $stack:ident (/ > $($tail:tt)*)) => {
        $crate::macros::child_to_parent(&mut $stack, None);
        html_impl! { $stack ($($tail)*) }
    };
    (@vtag $stack:ident ($($attr:ident)-+ = $val:expr, $($tail:tt)*)) => {
        let attr = local_vec![$(local_stringify!($attr).to_string()),+].join("-");
        $crate::macros::add_attribute(&mut $stack, &attr, $val);
        html_impl! { @vtag $stack ($($tail)*) }
    };
    // Close tag
    // eg: </$ident> ..
    ($stack:ident (< / $endtag:ident > $($tail:tt)*)) => {
        let endtag = local_stringify!($endtag);
        $crate::macros::child_to_parent(&mut $stack, Some(endtag));
        html_impl! { $stack ($($tail)*) }
    };
    // PATTERN: { for expression }
    // eg: { for $expr }
    ($stack:ident ({ for $eval:expr } $($tail:tt)*)) => {
        let nodes = $eval;
        let mut vlist = $crate::virtual_dom::VList::new();
        for node in nodes {
            let node = $crate::virtual_dom::VNode::from(node);
            vlist.add_child(node);
        }
        $stack.push(vlist.into());
        $crate::macros::child_to_parent(&mut $stack, None);
        html_impl! { $stack ($($tail)*) }
    };
    // Support root text nodes: #313
    // Provides `html!` blocks with only expression inside
    ($stack:ident ({ $eval:expr })) => {
        let node = $crate::virtual_dom::VNode::from($eval);
        $stack.push(node);
        html_impl! { $stack () }
    };
    // PATTERN: { expression }
    // eg: { $expr } ..
    ($stack:ident ({ $eval:expr } $($tail:tt)*)) => {
        let node = $crate::virtual_dom::VNode::from($eval);
        $crate::macros::add_child(&mut $stack, node);
        html_impl! { $stack ($($tail)*) }
    };
    // "End of paring" rule
    ($stack:ident ()) => {
        $crate::macros::unpack($stack)
    };
    ($stack:ident $($tail:tt)*) => {
        compile_error!("You should use curly bracets for text nodes: <a>{ \"Link\" }</a>");
    };
}

/// This macro implements JSX-like templates.
#[macro_export]
macro_rules! html {
    ($($tail:tt)*) => {{
        let mut stack = Vec::new();
        $crate::html_impl! { stack ($($tail)*) }
    }};
}

type Stack<COMP> = Vec<VNode<COMP>>;

#[doc(hidden)]
pub fn unpack<COMP: Component>(mut stack: Stack<COMP>) -> VNode<COMP> {
    const LENGTH_ERROR: &'static str = "html must contain exactly one element!";

    if stack.len() != 1 {
        panic!(LENGTH_ERROR);
    }
    let mut node = stack.pop().expect(LENGTH_ERROR);

    // Recursively update tag namespaces
    // 1. a parent sets a child's namespace, overriding its parent's namespaces
    // 2. a child adopts its parent's namespace
    fn proliferate_namespaces<COMP: Component>(node: &mut VNode<COMP>, ns: Option<Cow<'static, str>>) {
        match node {
            VNode::VTag(ref mut tag) => {
                let mut current_ns = ns;
                if let Some(ref tag_ns) = tag.ns {
                    current_ns = Some(tag_ns.clone());
                } else if let Some(ref some_ns) = current_ns {
                    tag.ns = Some(some_ns.clone());
                }

                for mut child in &mut tag.childs {
                    proliferate_namespaces(&mut child, current_ns.clone());
                }
            },
            VNode::VList(ref mut list) => {
                for mut child in &mut list.childs {
                    proliferate_namespaces(&mut child, ns.clone());
                }
            },
            VNode::VText(_) | VNode::VComp(_) | VNode::VRef(_) => (),
        }
    };

    proliferate_namespaces(&mut node, None);
    node
}

/// This method uses `value` attribute of macro to set a value of `input` element
/// or set that attribute as is for other elements like: `option`, `progress`, etc.
#[doc(hidden)]
pub fn set_value_or_attribute<COMP: Component, T: ToString>(stack: &mut Stack<COMP>, value: T) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        if vtag.tag().eq_ignore_ascii_case("input")
        || vtag.tag().eq_ignore_ascii_case("textarea") {
            vtag.set_value(&value)
        } else {
            vtag.add_attribute("value", &value)
        }
    } else {
        panic!("no tag to set value: {}", value.to_string());
    }
}

#[doc(hidden)]
pub fn set_kind<COMP: Component, T: ToString>(stack: &mut Stack<COMP>, value: T) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.set_kind(&value);
    } else {
        panic!("no tag to set type: {}", value.to_string());
    }
}

#[doc(hidden)]
pub fn set_checked<COMP: Component>(stack: &mut Stack<COMP>, value: bool) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.set_checked(value);
    } else {
        panic!("no tag to set checked: {}", value);
    }
}

#[doc(hidden)]
pub fn set_namespace<COMP, NS>(stack: &mut Stack<COMP>, namespace: NS)
where
    COMP: Component,
    NS: Into<Option<Cow<'static, str>>> {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.set_namespace(namespace);
    } else {
        use std::ops::Deref;
        panic!("no tag to set namespace: {}", namespace.into().as_ref().map(|c| c.deref()).unwrap_or(""));
    }
}


#[doc(hidden)]
pub fn add_attribute<COMP: Component, T: ToString>(stack: &mut Stack<COMP>, name: &str, value: T) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.add_attribute(name, &value);
    } else {
        panic!("no tag to set attribute: {}", name);
    }
}

#[doc(hidden)]
pub fn append_class<COMP: Component, T: AsRef<str>>(stack: &mut Stack<COMP>, class: T) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.add_class(class.as_ref());
    } else {
        panic!("no tag to attach class: {}", class.as_ref());
    }
}

#[doc(hidden)]
pub fn set_classes<COMP: Component, T: AsRef<str>>(stack: &mut Stack<COMP>, classes: T) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.set_classes(classes.as_ref());
    } else {
        panic!("no tag to set classes: {}", classes.as_ref());
    }
}

#[doc(hidden)]
pub fn attach_listener<COMP: Component>(
    stack: &mut Stack<COMP>,
    listener: Box<dyn Listener<COMP>>,
) {
    if let Some(&mut VNode::VTag(ref mut vtag)) = stack.last_mut() {
        vtag.add_listener(listener);
    } else {
        panic!("no tag to attach listener: {:?}", listener);
    }
}

#[doc(hidden)]
pub fn add_child<COMP: Component>(stack: &mut Stack<COMP>, child: VNode<COMP>) {
    match stack.last_mut() {
        Some(&mut VNode::VTag(ref mut vtag)) => {
            vtag.add_child(child);
        }
        Some(&mut VNode::VList(ref mut vlist)) => {
            vlist.add_child(child);
        }
        _ => {
            panic!("parent must be a tag or a fragment to add the node: {:?}", child);
        }
    }
}

#[doc(hidden)]
pub fn child_to_parent<COMP: Component>(stack: &mut Stack<COMP>, endtag: Option<&'static str>) {
    if let Some(mut node) = stack.pop() {
        // Check the enclosing tag
        // TODO Check it during compilation. Possible?
        if let (&mut VNode::VTag(ref mut vtag), Some(endtag)) = (&mut node, endtag) {
            let starttag = vtag.tag();
            if !starttag.eq_ignore_ascii_case(endtag) {
                panic!("wrong closing tag: <{}> -> </{}>", starttag, endtag);
            }
        }
        // Push the popped element to the last in the stack
        if !stack.is_empty() {
            match stack.last_mut() {
                Some(&mut VNode::VTag(ref mut vtag)) => {
                    vtag.add_child(node);
                }
                Some(&mut VNode::VList(ref mut vlist)) => {
                    vlist.add_child(node);
                }
                _ => {
                    panic!("can't add child to this type of node");
                }
            }
        } else {
            // Keep the last node in the stack
            stack.push(node);
        }
    } else {
        panic!("redundant closing tag: {:?}", endtag);
    }
}
