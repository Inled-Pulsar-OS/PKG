#include "mayonnaise/widgets/form.hpp"

#include "mayonnaise/app.hpp"
#include "mayonnaise/painter.hpp"

namespace mayo {

FormCardView::FormCardView() : StackView(Axis::Vertical) {
    setSpacing(0);
    setMaxSize(kInfinity, std::nullopt);
}

void FormCardView::draw(Painter& p) {
    const Theme& t = theme();
    const auto& c = t.colors;
    const Rect r = frame();
    const float radius = 12;
    Color fill = t.isDark() ? Color::white(0.05f) : Color::black(0.028f);
    p.fillRoundedRect(r, radius, fill, c.separator.multipliedAlpha(0.6f), 0.5f);
    const auto& rows = children();
    const float px = 1.0f / p.scale();
    for (size_t i = 0; i + 1 < rows.size(); ++i) {
        Rect a = rows[i]->frame();
        p.fillRect({a.x + 12, p.snap(a.bottom() - px * 0.5f), a.width - 24, px}, c.separator);
    }
    paintChildren(p);
}

FormRow::FormRow(std::string title, const Element& control, std::string subtitle)
    : WidgetHandle(std::make_shared<StackView>(Axis::Horizontal)) {
    m_node->setAlignment(Alignment::Leading);
    m_node->setSpacing(12);
    m_node->setPadding(Insets(7, 12));
    m_node->setMinSize(std::nullopt, 38);
    if (subtitle.empty()) {
        m_node->addChild(Label(std::move(title)).node());
    } else {
        m_node->addChild(VStack({Label(std::move(title)),
                                 Label(std::move(subtitle)).font(TextStyle::Subheadline).secondary()})
                             .spacing(1)
                             .node());
    }
    m_node->addChild(Spacer().node());
    if (control.widget()) m_node->addChild(control.widget());
}

FormRow::FormRow(const Element& content) : WidgetHandle(std::make_shared<StackView>(Axis::Horizontal)) {
    m_node->setPadding(Insets(7, 12));
    m_node->setMinSize(std::nullopt, 38);
    m_node->setAlignment(Alignment::Leading);
    if (content.widget()) {
        content.widget()->setMaxSize(kInfinity, std::nullopt);
        m_node->addChild(content.widget());
    }
}

FormSection::FormSection(std::string title, std::initializer_list<FormRow> rows, std::string footer)
    : WidgetHandle(std::make_shared<StackView>(Axis::Vertical)) {
    m_node->setSpacing(8);
    if (!title.empty()) m_node->addChild(Label(std::move(title)).font(TextStyle::Headline).padding(Insets(0, 4)).node());
    auto card = std::make_shared<FormCardView>();
    for (const auto& r : rows) card->addChild(r.node());
    m_node->addChild(card);
    if (!footer.empty()) {
        m_node->addChild(Label(std::move(footer)).font(TextStyle::Subheadline).secondary().padding(Insets(0, 4)).node());
    }
}

Form::Form(std::initializer_list<Element> sections) : ContainerHandle(std::make_shared<StackView>(Axis::Vertical)) {
    m_node->setSpacing(22);
    m_node->setMaxSize(kInfinity, std::nullopt);
    addAll(sections);
}

}  // namespace mayo
