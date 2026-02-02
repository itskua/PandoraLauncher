use std::sync::Arc;

use bridge::{instance::InstanceID, message::MessageToBackend};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable, Icon, IconName, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    resizable::{ResizablePanelEvent, ResizableState, h_resizable, resizable_panel},
    scroll::ScrollableElement,
    sidebar::{SidebarFooter, Sidebar},
    v_flex,
};
use rand::Rng;
use schema::modrinth::ModrinthProjectType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;


use crate::{
    component::{
        menu::{MenuGroup, MenuGroupItem},
        page_path::PagePath,
    },
    entity::{
        instance::{
            InstanceAddedEvent, InstanceEntries, InstanceModifiedEvent,
            InstanceMovedToTopEvent, InstanceRemovedEvent,
        },
        DataEntities,
    },
    interface_config::InterfaceConfig,
    modals,
    pages::{
        instance::instance_page::{InstancePage, InstanceSubpageType},
        instances_page::InstancesPage,
        modrinth_page::ModrinthSearchPage,
        syncing_page::SyncingPage,
    },
    png_render_cache,
};

pub struct LauncherUI {
    data: DataEntities,
    page: LauncherPage,
    sidebar_state: Entity<ResizableState>,
    default_sidebar_width: f32,
    recent_instances: heapless::Vec<(InstanceID, SharedString), 3>,
    _instance_added_subscription: Subscription,
    _instance_modified_subscription: Subscription,
    _instance_removed_subscription: Subscription,
    _instance_moved_to_top_subscription: Subscription,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PageType {
    Instances,
    Syncing,
    Modrinth {
        installing_for: Option<InstanceID>,
        project_type: Option<ModrinthProjectType>,
    },
    InstancePage(InstanceID, InstanceSubpageType),
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializedPageType {
    #[default]
    Instances,
    Syncing,
    Modrinth {
        installing_for: Option<SharedString>,
    },
    InstancePage(SharedString),
}

#[derive(Clone)]
pub enum LauncherPage {
    Instances(Entity<InstancesPage>),
    Syncing(Entity<SyncingPage>),
    Modrinth {
        installing_for: Option<InstanceID>,
        page: Entity<ModrinthSearchPage>,
    },
    InstancePage(InstanceID, InstanceSubpageType, Entity<InstancePage>),
}

impl LauncherPage {
    pub fn into_any_element(self) -> AnyElement {
        match self {
            LauncherPage::Instances(e) => e.into_any_element(),
            LauncherPage::Syncing(e) => e.into_any_element(),
            LauncherPage::Modrinth { page, .. } => page.into_any_element(),
            LauncherPage::InstancePage(_, _, e) => e.into_any_element(),
        }
    }

    pub fn page_type(&self) -> PageType {
        match self {
            LauncherPage::Instances(_) => PageType::Instances,
            LauncherPage::Syncing(_) => PageType::Syncing,
            LauncherPage::Modrinth { installing_for, .. } => {
                PageType::Modrinth { installing_for: *installing_for, project_type: None }
            }
            LauncherPage::InstancePage(id, sub, _) => PageType::InstancePage(*id, *sub),
        }
    }
}

impl LauncherUI {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar_state = cx.new(|_| ResizableState::default());

        cx.subscribe::<_, ResizablePanelEvent>(&sidebar_state, |this, state, event, cx| {
            if let ResizablePanelEvent::Resized = event {
                let sizes = state.read(cx).sizes();
                if let Some(size) = sizes.first() {
                    let w = size.to_f64() as f32;
                    InterfaceConfig::get_mut(cx).sidebar_width = w;
                    this.default_sidebar_width = w;
                }
            }
        })
        .detach();

        let recent_instances = data
            .instances
            .read(cx)
            .entries
            .iter()
            .take(3)
            .map(|(id, ent)| (*id, ent.read(cx).name.clone()))
            .collect();

        let _instance_added_subscription =
            cx.subscribe::<_, InstanceAddedEvent>(&data.instances, |this, _, ev, cx| {
                if this.recent_instances.is_full() {
                    this.recent_instances.pop();
                }
                let _ = this.recent_instances.insert(0, (ev.instance.id, ev.instance.name.clone()));
                cx.notify();
            });

        let _instance_modified_subscription =
            cx.subscribe::<_, InstanceModifiedEvent>(&data.instances, |this, _, ev, cx| {
                if let Some((_, name)) = this.recent_instances.iter_mut().find(|(id, _)| *id == ev.instance.id) {
                    *name = ev.instance.name.clone();
                }
                cx.notify();
            });

        let _instance_removed_subscription =
            cx.subscribe_in::<_, InstanceRemovedEvent>(&data.instances, window, |this, _, ev, window, cx| {
                this.recent_instances.retain(|(id, _)| *id != ev.id);
                if matches!(this.page, LauncherPage::InstancePage(id, _, _) if id == ev.id) {
                    this.switch_page(PageType::Instances, &[], window, cx);
                }
                cx.notify();
            });

        let _instance_moved_to_top_subscription =
            cx.subscribe::<_, InstanceMovedToTopEvent>(&data.instances, |this, _, ev, cx| {
                this.recent_instances.retain(|(id, _)| *id != ev.instance.id);
                if this.recent_instances.is_full() {
                    this.recent_instances.pop();
                }
                let _ = this.recent_instances.insert(0, (ev.instance.id, ev.instance.name.clone()));
                cx.notify();
            });

        let cfg = InterfaceConfig::get(cx);
        let page = PageType::from_serialized(&cfg.main_page, data, cx);
        let path = cfg.page_path.iter().map(|p| PageType::from_serialized(p, data, cx)).collect::<Vec<_>>();

        let mut sidebar_width = cfg.sidebar_width;
        if sidebar_width <= 0.0 {
            sidebar_width = 150.0;
        }

        Self {
            data: data.clone(),
            page: Self::create_page(data, page, &path, window, cx),
            sidebar_state,
            default_sidebar_width: sidebar_width,
            recent_instances,
            _instance_added_subscription,
            _instance_modified_subscription,
            _instance_removed_subscription,
            _instance_moved_to_top_subscription,
        }
    }

    fn create_page(
        data: &DataEntities,
        page: PageType,
        path: &[PageType],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> LauncherPage {
        let path = PagePath::new(path.iter().cloned().chain(std::iter::once(page)).collect());
        match page {
            PageType::Instances => LauncherPage::Instances(cx.new(|cx| InstancesPage::new(data, window, cx))),
            PageType::Syncing => LauncherPage::Syncing(cx.new(|cx| SyncingPage::new(data, window, cx))),
            PageType::Modrinth { installing_for, project_type } => {
                let page = cx.new(|cx| ModrinthSearchPage::new(installing_for, project_type, path, data, window, cx));
                LauncherPage::Modrinth { installing_for, page }
            }
            PageType::InstancePage(id, sub) => LauncherPage::InstancePage(
                id,
                sub,
                cx.new(|cx| InstancePage::new(id, sub, path, data, window, cx)),
            ),
        }
    }

    pub fn switch_page(
        &mut self,
        page: PageType,
        breadcrumbs: &[PageType],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page.page_type() == page {
            return;
        }

        let cfg = InterfaceConfig::get_mut(cx);
        cfg.main_page = page.to_serialized(&self.data, cx);
        cfg.page_path = breadcrumbs.iter().map(|p| p.to_serialized(&self.data, cx)).collect();

        self.page = Self::create_page(&self.data, page, breadcrumbs, window, cx);
        cx.notify();
    }
}

impl Render for LauncherUI {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page_type = self.page.page_type();

        let mut groups: heapless::Vec<MenuGroup, 3> = heapless::Vec::new();

        let _ = groups.push(
            MenuGroup::new("Play").child(
                MenuGroupItem::new("Instances")
                    .active(page_type == PageType::Instances)
                    .on_click(cx.listener(|ui, _, w, cx| ui.switch_page(PageType::Instances, &[], w, cx))),
            ),
        );

        let _ = groups.push(
            MenuGroup::new("Content")
                .child(
                    MenuGroupItem::new("Modrinth")
                        .active(matches!(page_type, PageType::Modrinth { .. }))
                        .on_click(cx.listener(|ui, _, w, cx| {
                            ui.switch_page(PageType::Modrinth { installing_for: None, project_type: None }, &[], w, cx)
                        })),
                )
                .child(
                    MenuGroupItem::new("Syncing")
                        .active(page_type == PageType::Syncing)
                        .on_click(cx.listener(|ui, _, w, cx| ui.switch_page(PageType::Syncing, &[], w, cx))),
                ),
        );

        let footer = h_flex().justify_center().w_full();

        let lumina_icon = Icon::empty().path("icons/lumina.svg");

        let sidebar = Sidebar::left()
            .w(relative(1.))
            .border_0()
            .header(
                h_flex()
                    .p_2()
                    .gap_2()
                    .w_full()
                    .justify_center()
                    .child(lumina_icon.size_8())
                    .child("LuminaForge"),
            )
            .footer(footer)
            .children(groups);

        h_resizable("container")
            .with_state(&self.sidebar_state)
            .child(resizable_panel().size(px(self.default_sidebar_width)).size_range(px(130.)..px(220.)).child(sidebar))
            .child(self.page.clone().into_any_element())
    }
}
