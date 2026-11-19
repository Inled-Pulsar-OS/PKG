#pragma once

#include "../app.hpp"
#include "../control.hpp"
#include "form.hpp"

namespace mayo {

/// Auto / Light / Dark choice drawn as three small window previews (System Settings style). Applies and
/// saves the choice through the App immediately.
class AppearancePickerView : public ControlView {
public:
    explicit AppearancePickerView(App& app) : m_app(app) {}
    Size measureContent(Size available) override;
    void draw(Painter& painter) override;
    void drawOverlay(Painter& painter) override;
    bool onPointerDown(const PointerEvent& e) override;
    void onPointerUp(const PointerEvent& e) override;
    void onPointerMove(const PointerEvent& e) override;
    void onPointerLeave() override;
    bool onKeyDown(const KeyEvent& e) override;

private:
    [[nodiscard]] Rect tileRect(int i) const;
    [[nodiscard]] int tileAt(Point p) const;
    App& m_app;
    int m_hover = -1;
    int m_pressed = -1;
};

/// Accent colour dots: the desktop's colour ("System") followed by the macOS palette, with the selected
/// colour's name underneath.
class AccentPickerView : public ControlView {
public:
    explicit AccentPickerView(App& app) : m_app(app) {}
    Size measureContent(Size available) override;
    void draw(Painter& painter) override;
    void drawOverlay(Painter& painter) override;
    bool onPointerDown(const PointerEvent& e) override;
    void onPointerUp(const PointerEvent& e) override;
    void onPointerMove(const PointerEvent& e) override;
    void onPointerLeave() override;
    bool onKeyDown(const KeyEvent& e) override;

private:
    [[nodiscard]] Point dotCenter(int i) const;
    [[nodiscard]] int dotAt(Point p) const;
    [[nodiscard]] int selectedIndex() const;  // 0 = System
    void choose(int i);
    App& m_app;
    int m_hover = -1;
    int m_pressed = -1;
};

class AppearancePicker : public WidgetHandle<AppearancePicker, AppearancePickerView> {
public:
    explicit AppearancePicker(App& app) : WidgetHandle(std::make_shared<AppearancePickerView>(app)) {}
};

class AccentPicker : public WidgetHandle<AccentPicker, AccentPickerView> {
public:
    explicit AccentPicker(App& app) : WidgetHandle(std::make_shared<AccentPickerView>(app)) {}
};

/// Ready-made "Appearance" form section (appearance + accent colour) for an app's settings window.
FormSection AppearanceSettings(App& app);

}  // namespace mayo
