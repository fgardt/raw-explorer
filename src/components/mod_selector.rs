use std::collections::BTreeSet;

use leptos::prelude::*;

use crate::util::fetch_from_resolver;

#[component]
pub fn ModSelector(
    selected_mod: WriteSignal<Option<String>>,
    base_version: WriteSignal<Option<String>>,
) -> impl IntoView {
    let mods = LocalResource::new(|| fetch_from_resolver::<AvailableMods>("stats"));

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
                                        base_version.set(Some(version.clone()));
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
