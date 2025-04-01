use std::sync::Arc;

use fapi_diff::format::prototype::PrototypeDoc;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use leptos_use::{use_clipboard, UseClipboardReturn};

use crate::{
    components::{ModSelector, TypeDisplayMode, TypeDisplayModeSwitcher, TypeLink},
    util::{get_dump, DedupValue, TypeHelper},
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/raw-explorer.css"/>

        // sets the document title
        <Title text="data.raw explorer"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let (variant, set_variant) = RwSignal::new(None).split();
    let (base_version, set_base_version) = RwSignal::new(None).split();
    let type_mode = RwSignal::new(TypeDisplayMode::Normal);
    let dump = LocalResource::new(move || get_dump(variant));
    let api_docs = Resource::new(move || base_version.get(), get_api_docs);

    view! {
        <h1>"Factorio data.raw explorer"</h1>
        <p>"Select the dump variant to explore: "
            <ModSelector selected_mod=set_variant base_version=set_base_version />
        </p>
        <TypeDisplayModeSwitcher type_mode=type_mode />
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
          {move || Suspend::new(async move {
            match dump.await {
                Ok(None) => {
                    ().into_any()
                },
                Ok(Some(data)) => {
                    let doc = api_docs.get().and_then(|d| d.ok().flatten().map(TypeHelper::new));
                    view! {
                        <JsonViewer val=data doc=doc type_mode=type_mode.read_only() start_open=true/>
                    }.into_any()
                },
                Err(e) => view! { <p>{e}</p> }.into_any(),
            }
          })}
        </Suspense>
    }
}

#[component]
fn JsonViewer(
    #[prop(optional)] key: Arc<str>,
    val: DedupValue,
    #[prop(optional_no_strip)] doc: Option<TypeHelper>,
    type_mode: ReadSignal<TypeDisplayMode>,
    #[prop(optional)] start_open: bool,
) -> impl IntoView {
    let (open, set_open) = RwSignal::new(start_open).split();

    let row = match val {
        DedupValue::Null => view! { <JsonKV
                key=key
                class="null"
                doc=doc
                type_mode=type_mode
                val="null".into()
            />
        }
        .into_any(),
        DedupValue::Bool(b) => view! { <JsonKV
                key=key
                class="bool"
                doc=doc
                type_mode=type_mode
                val=b.to_string()
            />
        }
        .into_any(),
        DedupValue::Number(n) => view! { <JsonKV
            key=key
            class="number"
            doc=doc
            type_mode=type_mode
            val=n.to_string()
            />
        }
        .into_any(),
        DedupValue::String(s) => view! { <JsonKV
            key=key
            class="text"
            doc=doc
            type_mode=type_mode
            val=format!("\"{s}\"")
            />
        }
        .into_any(),
        DedupValue::Array(arr) => {
            let len = arr.len();
            let raw = arr.clone();
            let d = doc.clone();
            let children = move || {
                open.get().then(|| {
                    arr.iter()
                        .enumerate()
                        .map(|(idx, v)| {
                            view! {
                                <JsonViewer
                                    key=idx.to_string().into()
                                    doc=d.clone().map(|d| d.traverse_idx(idx, len))
                                    type_mode=type_mode
                                    val=v.clone()
                                />
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader
                    key=key
                    doc=doc
                    type_mode=type_mode
                    write=set_open
                    raw=DedupValue::Array(raw)
                />
                <div class="json-children">
                    {children}
                </div>
            }
            .into_any()
        }
        DedupValue::Object(obj) => {
            let raw = obj.clone();
            let d = doc.clone();
            let children = move || {
                open.get().then(|| {
                    obj.iter()
                        .map(|(k, v)| {
                            view! {
                                <JsonViewer
                                    key=k.clone()
                                    doc=d.clone().map(|d| d.traverse_prop(k))
                                    type_mode=type_mode
                                    val=v.clone()
                                />
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader
                    key=key
                    doc=doc
                    type_mode=type_mode
                    write=set_open
                    raw=DedupValue::Object(raw)
                />
                <div class="json-children">
                    {children}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div class="json-row" class:expanded=move || open.get()>
            {row}
        </div>
    }
}

#[component]
fn JsonKV(
    key: Arc<str>,
    class: &'static str,
    #[prop(optional_no_strip)] doc: Option<TypeHelper>,
    type_mode: ReadSignal<TypeDisplayMode>,
    #[prop(optional)] val: String,
) -> impl IntoView {
    let class = if class.is_empty() { "empty" } else { class };

    let val_and_type = view! {
        <span class=class>{val}</span>
        {
            move || match &doc {
                Some(doc) => view! { <TypeLink doc=doc.clone() type_mode=type_mode /> }.into_any(),
                None => ().into_any(),
            }
        }
    }
    .into_any();

    if key.is_empty() {
        return val_and_type;
    }

    view! {
        <span class="key">{key}": "</span>{val_and_type}
    }
    .into_any()
}

#[component]
fn JsonCollapsibleHeader(
    key: Arc<str>,
    #[prop(optional_no_strip)] doc: Option<TypeHelper>,
    type_mode: ReadSignal<TypeDisplayMode>,
    #[prop(optional)] val: String,
    write: WriteSignal<bool>,
    raw: DedupValue,
) -> impl IntoView {
    let UseClipboardReturn {
        is_supported, copy, ..
    } = use_clipboard();

    view! {
        <a on:click=move |_| write.update(|v| *v = !*v)>
            <span class="arrow"/>
            <JsonKV
                key=key
                class=""
                doc=doc
                type_mode=type_mode
                val=val
            />
        </a>
        <Show when=move || is_supported.get()>
            <button on:click={
                let copy = copy.clone();
                let raw = raw.clone(); // cloning DedupValue should be cheap since its mostly Arc internally
                move |_| copy(&serde_json::to_string_pretty(&raw).unwrap())
            }>
                <Icon icon={icondata::FiCopy} width="1.1rem" height="1.1rem" />
            </button>
        </Show>
    }
}

// api docs need to be fetched from the server side to avoid CORS issues :)
#[server]
pub async fn get_api_docs(
    base_version: Option<String>,
) -> Result<Option<PrototypeDoc>, ServerFnError> {
    let Some(base_version) = base_version else {
        return Ok(None);
    };

    let res = crate::util::fetch_data::<PrototypeDoc>(&format!(
        "https://lua-api.factorio.com/{base_version}/prototype-api.json"
    ))
    .await
    .map_err(ServerFnError::new)?;
    Ok(Some(res))
}
