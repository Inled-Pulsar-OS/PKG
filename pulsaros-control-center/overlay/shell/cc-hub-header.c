/* cc-hub-header.c
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#include <config.h>

#include "cc-hub-header.h"

struct _CcHubHeader {
    AdwBin parent_instance;

    GtkBox *content;
    GtkImage *icon_image;
    GtkLabel *title_label;
    GtkLabel *description_label;
    GtkLinkButton *link_button;

    gchar *icon_color;
    gchar *color_class;
    gboolean hero;
};

G_DEFINE_FINAL_TYPE (CcHubHeader, cc_hub_header, ADW_TYPE_BIN)

enum {
    PROP_0,
    PROP_ICON_NAME,
    PROP_TITLE,
    PROP_DESCRIPTION,
    PROP_LINK_LABEL,
    PROP_LINK_URI,
    PROP_ICON_COLOR,
    PROP_HERO,
    N_PROPS
};

static GParamSpec *properties[N_PROPS];

static void
update_link (CcHubHeader *self)
{
    const gchar *label = gtk_button_get_label (GTK_BUTTON (self->link_button));
    const gchar *uri = gtk_link_button_get_uri (self->link_button);

    gtk_widget_set_visible (GTK_WIDGET (self->link_button), label && *label && uri && *uri);
}

static void
set_icon_color (CcHubHeader *self, const gchar *value)
{
    GdkRGBA rgba;

    if (self->color_class) {
        gtk_widget_remove_css_class (GTK_WIDGET (self->icon_image), self->color_class);
        g_clear_pointer (&self->color_class, g_free);
    }

    g_free (self->icon_color);
    self->icon_color = g_strdup (value);

    if (!value || !*value)
        return;

    if (gdk_rgba_parse (&rgba, value)) {
        /* CSS color: one rule per distinct color, shared by every header */
        static GHashTable *classes;
        g_autofree gchar *color = gdk_rgba_to_string (&rgba);
        const gchar *name;

        if (!classes)
            classes = g_hash_table_new_full (g_str_hash, g_str_equal, g_free, g_free);
        name = g_hash_table_lookup (classes, color);
        if (!name) {
            g_autofree gchar *css = NULL;
            GtkCssProvider *provider = gtk_css_provider_new ();
            gchar *new_name = g_strdup_printf ("pulsar-hub-color-%u", g_hash_table_size (classes));

            css = g_strdup_printf (".%s { background-color: %s; }", new_name, color);
            gtk_css_provider_load_from_string (provider, css);
            gtk_style_context_add_provider_for_display (gdk_display_get_default (),
                                                        GTK_STYLE_PROVIDER (provider),
                                                        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
            g_object_unref (provider);
            g_hash_table_insert (classes, g_strdup (color), new_name);
            name = new_name;
        }
        self->color_class = g_strdup (name);
    } else {
        /* Otherwise a class name, such as pulsar-icon-privacy */
        self->color_class = g_strdup (value);
    }

    gtk_widget_add_css_class (GTK_WIDGET (self->icon_image), self->color_class);
}

static void
set_hero (CcHubHeader *self, gboolean hero)
{
    self->hero = hero;

    gtk_orientable_set_orientation (GTK_ORIENTABLE (self->content),
                                    hero ? GTK_ORIENTATION_VERTICAL : GTK_ORIENTATION_HORIZONTAL);
    gtk_image_set_pixel_size (self->icon_image, hero ? 56 : 22);
    gtk_widget_set_valign (GTK_WIDGET (self->icon_image), hero ? GTK_ALIGN_CENTER : GTK_ALIGN_START);
    gtk_widget_set_halign (GTK_WIDGET (self->link_button), hero ? GTK_ALIGN_CENTER : GTK_ALIGN_START);
    gtk_label_set_xalign (self->title_label, hero ? 0.5 : 0);
    gtk_label_set_xalign (self->description_label, hero ? 0.5 : 0);
    gtk_label_set_justify (self->description_label, hero ? GTK_JUSTIFY_CENTER : GTK_JUSTIFY_LEFT);

    if (hero)
        gtk_widget_add_css_class (GTK_WIDGET (self), "hero");
    else
        gtk_widget_remove_css_class (GTK_WIDGET (self), "hero");
}

static void
cc_hub_header_set_property (GObject *object, guint prop_id, const GValue *value, GParamSpec *pspec)
{
    CcHubHeader *self = CC_HUB_HEADER (object);
    const gchar *str = G_VALUE_HOLDS_STRING (value) ? g_value_get_string (value) : NULL;

    switch (prop_id) {
    case PROP_ICON_NAME:
        gtk_image_set_from_icon_name (self->icon_image, str);
        break;
    case PROP_TITLE:
        gtk_label_set_label (self->title_label, str ? str : "");
        break;
    case PROP_DESCRIPTION:
        gtk_label_set_label (self->description_label, str ? str : "");
        gtk_widget_set_visible (GTK_WIDGET (self->description_label), str && *str);
        break;
    case PROP_LINK_LABEL:
        gtk_button_set_label (GTK_BUTTON (self->link_button), str ? str : "");
        update_link (self);
        break;
    case PROP_LINK_URI:
        gtk_link_button_set_uri (self->link_button, str ? str : "");
        update_link (self);
        break;
    case PROP_ICON_COLOR:
        set_icon_color (self, str);
        break;
    case PROP_HERO:
        set_hero (self, g_value_get_boolean (value));
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}

static void
cc_hub_header_get_property (GObject *object, guint prop_id, GValue *value, GParamSpec *pspec)
{
    CcHubHeader *self = CC_HUB_HEADER (object);

    switch (prop_id) {
    case PROP_ICON_NAME:
        g_value_set_string (value, gtk_image_get_icon_name (self->icon_image));
        break;
    case PROP_TITLE:
        g_value_set_string (value, gtk_label_get_label (self->title_label));
        break;
    case PROP_DESCRIPTION:
        g_value_set_string (value, gtk_label_get_label (self->description_label));
        break;
    case PROP_LINK_LABEL:
        g_value_set_string (value, gtk_button_get_label (GTK_BUTTON (self->link_button)));
        break;
    case PROP_LINK_URI:
        g_value_set_string (value, gtk_link_button_get_uri (self->link_button));
        break;
    case PROP_ICON_COLOR:
        g_value_set_string (value, self->icon_color);
        break;
    case PROP_HERO:
        g_value_set_boolean (value, self->hero);
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
    }
}

static void
cc_hub_header_finalize (GObject *object)
{
    CcHubHeader *self = CC_HUB_HEADER (object);

    g_clear_pointer (&self->icon_color, g_free);
    g_clear_pointer (&self->color_class, g_free);

    G_OBJECT_CLASS (cc_hub_header_parent_class)->finalize (object);
}

static void
cc_hub_header_class_init (CcHubHeaderClass *klass)
{
    GObjectClass *object_class = G_OBJECT_CLASS (klass);
    GtkWidgetClass *widget_class = GTK_WIDGET_CLASS (klass);
    const GParamFlags flags = G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS;

    object_class->set_property = cc_hub_header_set_property;
    object_class->get_property = cc_hub_header_get_property;
    object_class->finalize = cc_hub_header_finalize;

    properties[PROP_ICON_NAME] = g_param_spec_string ("icon-name", NULL, NULL, NULL, flags);
    properties[PROP_TITLE] = g_param_spec_string ("title", NULL, NULL, NULL, flags);
    properties[PROP_DESCRIPTION] = g_param_spec_string ("description", NULL, NULL, NULL, flags);
    properties[PROP_LINK_LABEL] = g_param_spec_string ("link-label", NULL, NULL, NULL, flags);
    properties[PROP_LINK_URI] = g_param_spec_string ("link-uri", NULL, NULL, NULL, flags);
    properties[PROP_ICON_COLOR] = g_param_spec_string ("icon-color", NULL, NULL, NULL, flags);
    properties[PROP_HERO] = g_param_spec_boolean ("hero", NULL, NULL, FALSE, flags);
    g_object_class_install_properties (object_class, N_PROPS, properties);

    gtk_widget_class_set_template_from_resource (widget_class, "/org/gnome/Settings/gtk/cc-hub-header.ui");

    gtk_widget_class_bind_template_child (widget_class, CcHubHeader, content);
    gtk_widget_class_bind_template_child (widget_class, CcHubHeader, icon_image);
    gtk_widget_class_bind_template_child (widget_class, CcHubHeader, title_label);
    gtk_widget_class_bind_template_child (widget_class, CcHubHeader, description_label);
    gtk_widget_class_bind_template_child (widget_class, CcHubHeader, link_button);
}

static void
cc_hub_header_init (CcHubHeader *self)
{
    gtk_widget_init_template (GTK_WIDGET (self));
    gtk_widget_set_visible (GTK_WIDGET (self->link_button), FALSE);
    gtk_widget_set_visible (GTK_WIDGET (self->description_label), FALSE);
}
