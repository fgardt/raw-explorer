use std::sync::Arc;

use fapi_diff::format::prototype::PrototypeDoc;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{components::*, hooks::use_params, params::Params, path};
use leptos_use::{UseClipboardReturn, use_clipboard};

use crate::{
    components::{GitHubCorner, ModSelector, TypeDisplayMode, TypeDisplayModeSwitcher, TypeLink},
    util::{DedupValue, TypeHelper, get_dump},
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
        <Stylesheet id="leptos" href="/pkg/raw-explorer.css"/>
        <Title text="data.raw explorer"/>
        <GitHubCorner repo="fgardt/raw-explorer"/>
        <h1>"Factorio data.raw explorer"</h1>
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage />
                    <ParentRoute path=path!("/e") view=VariantSelector>
                        <Route path=path!(":variant") view=Explorer />
                    </ParentRoute>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <ModSelector/>
        <p>"Select a mod to explore its data.raw dump"</p>
    }
}

#[derive(Params, PartialEq)]
pub struct VariantParams {
    pub variant: Option<String>,
}

#[component]
fn VariantSelector() -> impl IntoView {
    view! {
        <ModSelector/>
        <Outlet/>
    }
}

#[component]
fn Explorer() -> impl IntoView {
    let params = use_params::<VariantParams>();
    let variant = move || {
        params
            .read()
            .as_ref()
            .ok()
            .and_then(|p| p.variant.clone())
            .expect("variant is required")
    };
    let dump = LocalResource::new(move || get_dump(variant()));

    let type_mode = RwSignal::new(TypeDisplayMode::Normal);
    let api_docs = Resource::new(|| (), async |_| get_api_docs().await);

    view! {
        <TypeDisplayModeSwitcher type_mode=type_mode />
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
          {move || Suspend::new(async move {
            match dump.await {
                Ok(data) => {
                    let doc = api_docs.get().and_then(|d| d.ok().map(TypeHelper::new));
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
        <div class="json-row" class:expanded=open>
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
                <Icon icon={icondata::FiCopy} width="1rem" height="1rem" />
            </button>
        </Show>
    }
}

// api docs need to be fetched from the server side to avoid CORS issues :)
#[server]
pub async fn get_api_docs() -> Result<PrototypeDoc, ServerFnError> {
    #[derive(serde::Deserialize)]
    struct ProcessedMods {
        #[serde(rename = "processed")]
        raw: std::collections::BTreeSet<String>,
    }

    let base_version = crate::util::fetch_from_resolver::<ProcessedMods>("stats")
        .await
        .map_err(|_| ())
        .and_then(|a| {
            a.raw
                .iter()
                .find_map(|r| {
                    if r.starts_with("base_") {
                        Some(r.trim_start_matches("base_").to_string())
                    } else {
                        None
                    }
                })
                .ok_or(())
        })
        .unwrap_or_else(|()| "latest".to_string());

    crate::util::fetch_data(&format!(
        "https://lua-api.factorio.com/{base_version}/prototype-api.json"
    ))
    .await
    .map_err(ServerFnError::new)
}
