use std::{collections::BTreeSet, sync::Arc};

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
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
    let dump = LocalResource::new(move || get_dump(variant));

    view! {
        <h1>"Factorio data.raw explorer"</h1>
        <p>"Select the dump variant to explore: "
            <ModSelector selected_mod=set_variant />
        </p>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
          {move || Suspend::new(async move {
            match dump.await {
                Ok(None) => {
                    ().into_any()
                },
                Ok(Some(data)) => {
                    view! {
                        <JsonViewer val=data start_open=true/>
                    }.into_any()
                },
                Err(e) => view! { <p>{e}</p> }.into_any(),
            }
          })}
        </Suspense>
    }
}

use crate::util::DedupValue;

#[component]
fn JsonViewer(
    #[prop(optional)] key: Arc<str>,
    val: DedupValue,
    #[prop(optional)] start_open: bool,
) -> impl IntoView {
    let (open, set_open) = RwSignal::new(start_open).split();

    let row = match val {
        DedupValue::Null => view! { <JsonKV key=key kind="null" val="null".into()/> }.into_any(),
        DedupValue::Bool(b) => {
            view! { <JsonKV key=key kind="bool" val=b.to_string() /> }.into_any()
        }
        DedupValue::Number(n) => {
            view! { <JsonKV key=key kind="number" val=n.to_string() /> }.into_any()
        }
        DedupValue::String(s) => {
            view! { <JsonKV key=key kind="text" val=format!("\"{s}\"") /> }.into_any()
        }
        DedupValue::Array(arr) => {
            let len = arr.len();
            let children = move || {
                open.get().then(|| {
                    arr.iter()
                        .enumerate()
                        .map(|(idx, v)| {
                            view! {
                                <JsonViewer key=idx.to_string().into() val=v.clone()/>
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader key=key kind="array" val=len.to_string() write=set_open />
                <div class="json-children">
                    {children}
                </div>
            }
            .into_any()
        }
        DedupValue::Object(obj) => {
            let children = move || {
                open.get().then(|| {
                    obj.iter()
                        .map(|(k, v)| {
                            view! {
                                <JsonViewer key=k.clone() val=v.clone()/>
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader key=key kind="object" write=set_open />
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
fn JsonKV(key: Arc<str>, kind: &'static str, #[prop(optional)] val: String) -> impl IntoView {
    let val = view! {
        <span class=kind>{val}</span>
    }
    .into_any();

    if key.is_empty() {
        return val;
    }

    view! {
        <span class="key">{key}": "</span>{val}
    }
    .into_any()
}

#[component]
fn JsonCollapsibleHeader(
    key: Arc<str>,
    kind: &'static str,
    #[prop(optional)] val: String,
    write: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <a on:click=move |_| write.update(|v| *v = !*v)>
            <span class="arrow"/>
            <JsonKV key=key kind=kind val=val/>
        </a>
    }
}

#[component]
fn ModSelector(selected_mod: WriteSignal<Option<String>>) -> impl IntoView {
    let mods = LocalResource::new(|| get_from_resolver::<AvailableMods>("stats"));

    view! {
        <Suspense fallback=move || view! {
            <select>
                <option value="" disabled selected>"Loading available mods.."</option>
            </select>
        }>
            <select on:change:target=move |ev| {
                let m = ev.target().value();
                let val = if m.is_empty() { None } else { Some(m) };

                selected_mod.set(val);
            }>
                <option value="" disabled>"Select a mod"</option>
                {move || Suspend::new(async move {
                    match mods.await {
                        Ok(mods) => {
                            mods.build_list().iter().enumerate().map(|(idx, (name, version))| {
                                    if idx == 0 {
                                        selected_mod.set(Some(name.clone()));
                                    }

                                    view! {
                                        <option value={name.clone()}>{name.clone()} " (" {version.clone()} ")"</option>
                                    }
                                }).collect_view().into_any()
                        }
                        Err(e) => {
                            view! {
                                <option value="" disabled selected>{e}</option>
                            }.into_any()
                        }
                    }
                })}
            </select>
        </Suspense>
    }
}

#[derive(serde::Deserialize, Clone)]
struct AvailableMods {
    #[serde(rename = "processed")]
    raw: BTreeSet<String>,
}

impl AvailableMods {
    fn build_list(&self) -> Box<[(String, String)]> {
        const WUBE_MODS: [&str; 4] = ["base", "space-age", "quality", "elevated-rails"];

        let mut split = self
            .raw
            .iter()
            .filter_map(|r| {
                let mut parts = r.split('_').collect::<Vec<_>>();
                if parts.len() < 2 {
                    return None;
                }

                let version = parts.pop()?.to_string();
                let name = parts.join("_");

                Some((name, version))
            })
            .collect::<Vec<_>>();

        split.sort_by(|(a, _), (b, _)| {
            let a_wube = WUBE_MODS.iter().position(|&m| m == a).unwrap_or(usize::MAX);
            let b_wube = WUBE_MODS.iter().position(|&m| m == b).unwrap_or(usize::MAX);

            match a_wube.cmp(&b_wube) {
                std::cmp::Ordering::Equal => a.cmp(b),
                other => other,
            }
        });
        split.into_boxed_slice()
    }
}

pub async fn get_from_resolver<T: serde::de::DeserializeOwned>(uri: &str) -> Result<T, String> {
    let resp = reqwest::Client::new()
        .get(format!("https://modname_resolver.bpbin.com/{uri}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let msg = resp.text().await.map_err(|e| e.to_string())?;

        if msg.is_empty() {
            return Err(format!("Request failed with status: {status}"));
        } else {
            return Err(format!("Request failed ({status}): {msg}"));
        }
    }

    let json = resp.json::<T>().await.map_err(|e| e.to_string())?;
    Ok(json)
}

pub async fn get_dump(variant: ReadSignal<Option<String>>) -> Result<Option<DedupValue>, String> {
    let Some(variant) = variant.get() else {
        return Ok(None);
    };

    let res = get_from_resolver(&format!("raw/{variant}")).await?;
    Ok(Some(res))
}
