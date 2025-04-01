use std::sync::Arc;

use fapi_diff::format::prototype::{ComplexType, LiteralValue, Type};
use leptos::prelude::*;

use crate::util::{CurrentType, TypeHelper};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeDisplayMode {
    Normal,
    All,
    Off,
    Debug,
}

impl TypeDisplayMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Normal => Self::All,
            Self::All => Self::Off,
            #[cfg(not(debug_assertions))]
            Self::Off => Self::Normal,
            #[cfg(debug_assertions)]
            Self::Off => Self::Debug,
            Self::Debug => Self::Normal,
        }
    }
}

impl std::fmt::Display for TypeDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "Normal"),
            Self::All => write!(f, "All"),
            Self::Off => write!(f, "Off"),
            Self::Debug => write!(f, "Debug"),
        }
    }
}

#[component]
pub fn TypeDisplayModeSwitcher(type_mode: RwSignal<TypeDisplayMode>) -> impl IntoView {
    view! {
        <p>"Type display mode: "
        {
            #[cfg(not(debug_assertions))]
            const MODES: [TypeDisplayMode; 3] = [TypeDisplayMode::Normal, TypeDisplayMode::All, TypeDisplayMode::Off];
            #[cfg(debug_assertions)]
            const MODES: [TypeDisplayMode; 4] = [TypeDisplayMode::Normal, TypeDisplayMode::All, TypeDisplayMode::Off, TypeDisplayMode::Debug];

            MODES.iter().map(|m| {
                view! {
                    <button
                        on:click=move |_| type_mode.update(|cm| *cm = *m)
                        class="type-display-mode"
                        class:active=move || *m == type_mode.get()
                        disabled=move || *m == type_mode.get()
                    >
                        {format!("{m}")}
                    </button>
                }
            }).collect_view()
        }
        </p>
    }
}

#[component]
pub fn TypeLink(doc: TypeHelper, type_mode: ReadSignal<TypeDisplayMode>) -> impl IntoView {
    view! {
        <TypeLinkInternal doc=doc type_mode=type_mode />
    }
}

#[component]
fn DocLink(doc: TypeHelper, target: Arc<str>) -> impl IntoView {
    let name = target.clone();
    view! {
        <a class="doc-link" href={doc.get_doc_link(name)} target="_blank">
            {target}
        </a>
    }
}

#[component]
fn TypeLinkInternal(doc: TypeHelper, type_mode: ReadSignal<TypeDisplayMode>) -> impl IntoView {
    use CurrentType::*;
    use TypeDisplayMode::*;

    let d = doc.clone();
    let kind = doc.kind;

    view! {
        {
            move || match (kind.clone(), type_mode.get()) {
                (Unknown, All | Debug) => view! { <span class="type">"?"</span> }.into_any(),
                (d, Debug) => view! { <span class="type">{format!("{d:?}")}</span> }.into_any(),
                (DataRaw, _) => view! { <span class="type">"data.raw"</span> }.into_any(),
                (TypeOrProto(name), Normal | All) => view!{
                    <span class="type">
                        <DocLink doc=d.clone() target=name.clone() />
                    </span>
                }.into_any(),
                (BuiltIn(name), All) => view! {
                    <span class="type">
                        <DocLink doc=d.clone() target=name.clone() />
                    </span>
                }.into_any(),
                (Complex(c), Normal | All) => view! {
                    <ComplexTypeLink doc=d.clone() complex=c.clone() type_mode=type_mode />
                }.into_any(),
                _ => ().into_any(),
            }
        }
    }
}

#[component]
fn ComplexTypeLink(
    doc: TypeHelper,
    complex: ComplexType,
    type_mode: ReadSignal<TypeDisplayMode>,
) -> impl IntoView {
    use ComplexType::*;
    use TypeDisplayMode::*;

    let d = doc.clone();

    view! {
        {
            move || match (complex.clone(), type_mode.get()) {
                (Array { value }, Normal | All | Debug) => view! {
                    <span class="type array">
                        <ComplexLinkInternal doc=d.clone() complex=value.clone() type_mode=type_mode />
                    </span>
                }.into_any(),
                (Dictionary { key, value }, Normal | All | Debug) => view! {
                    <span class="type dictionary">
                        <ComplexLinkInternal doc=d.clone() complex=key.clone() type_mode=type_mode />
                        <ComplexLinkInternal doc=d.clone() complex=value.clone() type_mode=type_mode />
                    </span>
                }.into_any(),
                (Tuple{ values }, Normal | All | Debug) => view! {
                    <span class="type tuple">
                        {
                            values
                                .iter()
                                .map(|v| {
                                    view!{
                                        <ComplexLinkInternal
                                            doc=d.clone()
                                            complex=v.clone()
                                            type_mode=type_mode
                                        />
                                    }
                                })
                                .collect_view()
                        }
                    </span>
                }.into_any(),
                (Union{ options, ..}, Normal | All | Debug) => view! {
                    <span class="type union">
                        {
                            options
                                .iter()
                                .map(|o| {
                                    view!{
                                        <ComplexLinkInternal
                                            doc=d.clone()
                                            complex=o.clone()
                                            type_mode=type_mode
                                        />
                                    }
                                })
                                .collect_view()
                        }
                    </span>
                }.into_any(),
                (Type{ value, .. }, Normal | All | Debug) => view! {
                    <ComplexLinkInternal doc=d.clone() complex=value.clone() type_mode=type_mode />
                }.into_any(),
                (Literal(literal), Normal | All | Debug) => {
                    use LiteralValue::*;
                    match literal.value {
                        Boolean(b) => view! { <span class="type bool">{b.to_string()}</span> }.into_any(),
                        UInt(u) => view! { <span class="type number">{u.to_string()}</span> }.into_any(),
                        Int(i) => view! { <span class="type number">{i.to_string()}</span> }.into_any(),
                        Float(f) => view! { <span class="type number">{f.to_string()}</span> }.into_any(),
                        String(s) => view! { <span class="type text">{s}</span> }.into_any(),
                    }
                },
                (_, Debug) => view! {
                    <span class="type">{format!("{complex:?}")}</span>
                }.into_any(),
                _ => ().into_any(),
            }
        }
    }
}

#[component]
fn ComplexLinkInternal(
    doc: TypeHelper,
    complex: Type,
    type_mode: ReadSignal<TypeDisplayMode>,
) -> impl IntoView {
    use Type::*;

    view! {
        {
            move || match complex.clone() {
                Simple(t) => view! {
                    <span>
                        <DocLink doc=doc.clone() target=t.clone().into() />
                    </span>
                }.into_any(),
                Complex(c) => view! {
                    <span>
                        <ComplexTypeLink doc=doc.clone() complex=*c.clone() type_mode=type_mode />
                    </span>
                }.into_any(),
            }
        }
    }
}
