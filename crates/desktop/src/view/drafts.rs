//! Draft input and review, without repeated instructional panels.
use super::controls::{self, Tone};
use super::{design, media, muted};
use crate::{app::Downloader, drafts::DraftState, settings::Page};
use gpui_kit::{
    component::{select::Select, *},
    *,
};
impl Downloader {
    pub(super) fn draft_home(&self, cx: &Context<Self>) -> Div {
        let mut panel = div().v_flex().gap_4().max_w(px(900.)).mx_auto().child(
            design::group(cx)
                .v_flex()
                .p_5()
                .gap_3()
                .bg(design::surface(cx))
                .child(muted(cx, crate::i18n::tr("视频链接")))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(controls::input(&self.url, cx)),
                        )
                        .child(
                            controls::icon_button(
                                "draft-paste",
                                IconName::Copy,
                                crate::i18n::tr("从剪贴板粘贴"),
                                cx,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    if let Some(text) =
                                        cx.read_from_clipboard().and_then(|i| i.text())
                                    {
                                        this.url.update(cx, |s, cx| s.set_value(text, window, cx));
                                    }
                                },
                            )),
                        )
                        .child(
                            controls::button("add-draft", "", Tone::Primary, cx)
                                .small()
                                .bg(design::accent())
                                .label(crate::i18n::tr("解析链接"))
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.add_draft(window, cx)),
                                ),
                        ),
                )
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_3()
                        .child(
                            muted(cx, crate::i18n::tr("下载配置"))
                                .w(px(96.))
                                .flex_shrink_0(),
                        )
                        .child(
                            div().w(px(280.)).min_w_0().child(
                                Select::new(&self.preset_select)
                                    .small()
                                    .placeholder(crate::i18n::tr("使用下载配置")),
                            ),
                        ),
                ),
        );
        let count = self
            .drafts
            .entries
            .iter()
            .filter(|d| d.state != DraftState::Queued)
            .count();
        if count > 0 {
            panel = panel.child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .pt_3()
                    .child(design::heading(crate::i18n::tr("待确认的下载")))
                    .child(muted(cx, count.to_string())),
            );
        }
        for d in self
            .drafts
            .entries
            .iter()
            .filter(|d| d.state != DraftState::Queued)
        {
            let id = d.id.clone();
            let title = d
                .media
                .as_ref()
                .map(|m| m.title.clone())
                .unwrap_or(d.url.clone());
            let retry_id = id.clone();
            let remove_id = id.clone();
            let review_id = id.clone();
            let mut info = div()
                .v_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(media::title(
                    SharedString::from(format!("draft-title-{}", d.id)),
                    title,
                    cx,
                ))
                .child(media::metadata(d.media.as_ref(), &d.profile, cx))
                .child(controls::badge(
                    draft_state(d.state),
                    d.state == DraftState::Ready,
                    cx,
                ));
            if let Some(error) = &d.error {
                info = info.child(muted(cx, error.clone()));
            }
            panel = panel.child(
                design::group(cx)
                    .id(SharedString::from(format!("draft-card-{}", d.id)))
                    .h_flex()
                    .items_center()
                    .gap_4()
                    .p_4()
                    .cursor_pointer()
                    .hover(|s| s.bg(design::canvas(cx)))
                    .child(design::media_tile(
                        cx,
                        d.profile.mode != youtube_engine::Mode::Video,
                    ))
                    .child(info)
                    .children((d.state == DraftState::Ready).then(|| {
                        controls::button(
                            SharedString::from(format!("review-draft-{}", d.id)),
                            crate::i18n::tr("查看并下载"),
                            Tone::Secondary,
                            cx,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.open_draft(&review_id, window, cx);
                            },
                        ))
                    }))
                    .children(
                        matches!(
                            d.state,
                            DraftState::Failed | DraftState::Interrupted | DraftState::Cancelled
                        )
                        .then(|| {
                            controls::icon_button(
                                SharedString::from(format!("retry-draft-{}", d.id)),
                                IconName::RefreshCw,
                                crate::i18n::tr("重新解析"),
                                cx,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.retry_draft(&retry_id, cx);
                                },
                            ))
                        }),
                    )
                    .child(
                        controls::icon_button(
                            SharedString::from(format!("remove-draft-{}", d.id)),
                            IconName::Close,
                            crate::i18n::tr("移除草稿"),
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.remove_draft(&remove_id, cx);
                        })),
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_draft(&id, window, cx)),
                    ),
            );
        }
        if count == 0 {
            panel = panel.child(
                div()
                    .v_flex()
                    .items_center()
                    .justify_center()
                    .py_12()
                    .gap_3()
                    .child(design::glyph("link"))
                    .child(muted(cx, crate::i18n::tr("暂无待确认的下载"))),
            );
        }
        panel
    }
    pub(super) fn draft_detail(&self, cx: &Context<Self>) -> Div {
        let draft = self
            .selected_draft
            .as_ref()
            .and_then(|id| self.drafts.entries.iter().find(|d| &d.id == id));
        let mut body = div().v_flex().gap_5().max_w(px(860.)).mx_auto();
        if let Some(d) = draft {
            let title = d
                .media
                .as_ref()
                .map(|m| m.title.clone())
                .unwrap_or(d.url.clone());
            let mut info = div()
                .v_flex()
                .flex_1()
                .min_w_0()
                .gap_3()
                .child(div().text_size(px(18.)).font_semibold().child(title))
                .child(media::metadata(
                    d.media.as_ref(),
                    &self.settings.snapshot(cx),
                    cx,
                ));
            if d.media.is_none() {
                info = info.child(muted(cx, draft_state(d.state)));
            }
            if let Some(error) = &d.error {
                info = info.child(muted(cx, error.clone()));
            }
            body = body.child(
                design::group(cx)
                    .h_flex()
                    .items_center()
                    .p_5()
                    .gap_4()
                    .child(design::media_tile(
                        cx,
                        d.profile.mode != youtube_engine::Mode::Video,
                    ))
                    .child(info),
            );
        }
        body.child(self.confirmation(cx))
    }
    pub(super) fn draft_options(&self, cx: &Context<Self>) -> Div {
        div()
            .v_flex()
            .gap_4()
            .max_w(px(860.))
            .mx_auto()
            .child(
                div().h_flex().justify_end().child(
                    controls::button("save-task-options", "", Tone::Primary, cx)
                        .small()
                        .bg(design::accent())
                        .label(crate::i18n::tr("保存并返回"))
                        .tooltip(crate::i18n::tr(
                            "仅作用于当前任务，不会修改常用配置或其他任务。",
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.save_draft_editor(cx) {
                                this.download(cx);
                            } else {
                                this.pending = None;
                            }
                            this.page = Page::Detail;
                            cx.notify();
                        })),
                ),
            )
            .child(self.profile_editor(cx))
    }
}
fn draft_state(state: DraftState) -> String {
    crate::i18n::tr(match state {
        DraftState::Waiting => "等待解析",
        DraftState::Probing => "正在解析",
        DraftState::Ready => "待确认",
        DraftState::Failed => "解析失败",
        DraftState::Cancelled => "已取消",
        DraftState::Interrupted => "已中断",
        DraftState::Queued => "等待下载",
    })
}
