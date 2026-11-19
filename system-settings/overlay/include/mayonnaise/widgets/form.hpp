#pragma once

#include <string>

#include "../layout.hpp"
#include "label.hpp"

namespace mayo {

/// Rounded card that stacks rows with inset hairline separators (System Settings grouped style).
class FormCardView : public StackView {
public:
    FormCardView();
    void draw(Painter& painter) override;
};

/// One labelled row: title (and optional subtitle) leading, control trailing.
class FormRow : public WidgetHandle<FormRow, StackView> {
public:
    FormRow(std::string title, const Element& control, std::string subtitle = {});
    /// Full-width custom row content.
    explicit FormRow(const Element& content);
};

class FormSection : public WidgetHandle<FormSection, StackView> {
public:
    FormSection(std::string title, std::initializer_list<FormRow> rows, std::string footer = {});
    FormSection(std::initializer_list<FormRow> rows) : FormSection({}, rows) {}  // NOLINT
};

class Form : public detail::ContainerHandle<Form, StackView> {
public:
    Form(std::initializer_list<Element> sections);  // NOLINT(google-explicit-constructor)
};

}  // namespace mayo
