//! Draft cards are independently addressable and never own the global editor.
use super::{design, muted};
use crate::{app::Downloader, drafts::DraftState, settings::Page};
use gpui_kit::{
    component::{button::*, input::Input, select::Select, *},
    *,
};
impl Downloader {
    pub(super) fn draft_home(&self, cx: &Context<Self>) -> Div {
        let mut panel = div()
            .v_flex()
            .gap_4()
            .max_w(px(900.))
            .mx_auto()
            .py_4()
            .child(
                design::section(cx, crate::i18n::tr("新建任务"))
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().min_w_0().child(Input::new(&self.url)))
                            .child(
                                Button::new("draft-paste")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Copy)
                                    .tooltip(crate::i18n::tr("从剪贴板粘贴"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(text) =
                                            cx.read_from_clipboard().and_then(|i| i.text())
                                        {
                                            this.url
                                                .update(cx, |s, cx| s.set_value(text, window, cx));
                                        }
                                    })),
                            ),
                    )
                    .child(
                        div().h_flex().gap_3().items_center().child(
                            div().flex_1().min_w_0().child(
                                Select::new(&self.preset_select)
                                    .placeholder(crate::i18n::tr("使用下载配置")),
                            ),
                        ),
                    )
                    .child(
                        Button::new("add-draft")
                            .primary()
                            .small()
                            .icon(IconName::Plus)
                            .label(crate::i18n::tr("解析链接"))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.add_draft(window, cx)),
                            ),
                    ),
            );
        for (index, d) in self
            .drafts
            .entries
            .iter()
            .filter(|d| d.state != DraftState::Queued)
            .enumerate()
        {
            let id = d.id.clone();
            let title = d
                .media
                .as_ref()
                .map(|m| m.title.clone())
                .unwrap_or(d.url.clone());
            let retry_id = id.clone();
            let remove_id = id.clone();
            let state = crate::i18n::tr(match d.state {
                DraftState::Waiting => "等待解析",
                DraftState::Probing => "正在解析",
                DraftState::Ready => "待确认",
                DraftState::Failed => "解析失败",
                DraftState::Cancelled => "已取消",
                DraftState::Interrupted => "已中断",
                DraftState::Queued => "等待下载",
            });
            panel = panel.child(
                div()
                    .id(("draft-card", index))
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .cursor_pointer()
                    .hover(|s| s.bg(cx.theme().muted))
                    .child(
                        div()
                            .h_flex()
                            .gap_3()
                            .child(design::glyph("video"))
                            .child(div().flex_1().min_w_0().overflow_hidden().child(title))
                            .child(muted(cx, state))
                            .children(
                                matches!(
                                    d.state,
                                    DraftState::Failed
                                        | DraftState::Interrupted
                                        | DraftState::Cancelled
                                )
                                .then(|| {
                                    Button::new(("retry-draft", index))
                                        .ghost()
                                        .small()
                                        .icon(IconName::RefreshCw)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            this.retry_draft(&retry_id, cx);
                                        }))
                                }),
                            )
                            .child(
                                Button::new(("remove-draft", index))
                                    .ghost()
                                    .small()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.remove_draft(&remove_id, cx);
                                    })),
                            ),
                    )
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_draft(&id, window, cx)),
                    ),
            );
        }
        panel
    }
    pub(super) fn draft_detail(&self, cx: &Context<Self>) -> Div {
        div()
            .v_flex()
            .gap_4()
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .child(
                        Button::new("draft-back")
                            .ghost()
                            .small()
                            .label(crate::i18n::tr("返回"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_draft_editor(cx);
                                this.page = Page::New;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("draft-review")
                            .primary()
                            .small()
                            .label(crate::i18n::tr("下一步：查看命令"))
                            .disabled(self.media.is_none())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.save_draft_editor(cx) {
                                    this.download(cx);
                                }
                            })),
                    ),
            )
            .child(
                self.media
                    .as_ref()
                    .map(|m| m.title.clone())
                    .unwrap_or_else(|| "解析尚未完成".into()),
            )
            .child(self.profile_editor(cx))
    }
}
