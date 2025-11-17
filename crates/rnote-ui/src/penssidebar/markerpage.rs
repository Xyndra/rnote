// Imports
use crate::RnAppWindow;
use adw::prelude::*;
use gtk4::{
    Adjustment, Button, CompositeTemplate, ListBox, MenuButton, Widget, glib, glib::clone,
    subclass::prelude::*,
};
use rnote_engine::pens::pensconfig::markerconfig::MarkerShape;

mod imp {
    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/markerpage.ui")]
    pub(crate) struct RnMarkerPage {
        #[template_child]
        pub(crate) markerconfig_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) markerconfig_popover_close_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) stroke_width_picker:
            TemplateChild<crate::strokewidthpicker::RnStrokeWidthPicker>,
        #[template_child]
        pub(crate) marker_shape_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) shape_circular_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) shape_rectangular_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(crate) strength_adj: TemplateChild<Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnMarkerPage {
        const NAME: &'static str = "RnMarkerPage";
        type Type = super::RnMarkerPage;
        type ParentType = Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnMarkerPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            self.dispose_template();
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for RnMarkerPage {}
}

glib::wrapper! {
    pub(crate) struct RnMarkerPage(ObjectSubclass<imp::RnMarkerPage>)
        @extends Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for RnMarkerPage {
    fn default() -> Self {
        Self::new()
    }
}

impl RnMarkerPage {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();

        // Close button for config popover
        imp.markerconfig_popover_close_button
            .connect_clicked(clone!(
                #[weak(rename_to = menubutton)]
                imp.markerconfig_menubutton,
                move |_| {
                    menubutton.popdown();
                }
            ));

        // Stroke width picker
        imp.stroke_width_picker.spinbutton().set_range(
            rnote_engine::pens::pensconfig::markerconfig::MarkerConfig::WIDTH_MIN,
            rnote_engine::pens::pensconfig::markerconfig::MarkerConfig::WIDTH_MAX,
        );
        imp.stroke_width_picker.set_stroke_width(
            rnote_engine::pens::pensconfig::markerconfig::MarkerConfig::default().width,
        );

        imp.stroke_width_picker.connect_notify_local(
            Some("stroke-width"),
            clone!(
                #[weak]
                appwindow,
                move |picker, _| {
                    let width = picker.stroke_width();
                    appwindow
                        .engine_config()
                        .write()
                        .pens_config
                        .marker_config
                        .width = width;
                }
            ),
        );

        // Shape listbox
        imp.marker_shape_listbox.connect_row_selected(clone!(
            #[weak]
            appwindow,
            #[weak(rename_to = circular_row)]
            imp.shape_circular_row,
            #[weak(rename_to = rectangular_row)]
            imp.shape_rectangular_row,
            move |_listbox, selected_row| {
                if let Some(selected_row) = selected_row {
                    if selected_row == circular_row.upcast_ref::<gtk4::ListBoxRow>() {
                        appwindow
                            .engine_config()
                            .write()
                            .pens_config
                            .marker_config
                            .shape = MarkerShape::Circular;
                    } else if selected_row == rectangular_row.upcast_ref::<gtk4::ListBoxRow>() {
                        appwindow
                            .engine_config()
                            .write()
                            .pens_config
                            .marker_config
                            .shape = MarkerShape::Rectangular;
                    }
                }
            }
        ));

        // Strength adjustment
        imp.strength_adj.connect_value_changed(clone!(
            #[weak]
            appwindow,
            move |strength_adj| {
                let strength = strength_adj.value() / 100.0;

                appwindow
                    .engine_config()
                    .write()
                    .pens_config
                    .marker_config
                    .strength = strength;
            }
        ));
    }

    pub(crate) fn refresh_ui(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();
        let marker_config = appwindow
            .engine_config()
            .read()
            .pens_config
            .marker_config
            .clone();

        // Update stroke width picker
        imp.stroke_width_picker
            .set_stroke_width(marker_config.width);

        // Update strength
        imp.strength_adj.set_value(marker_config.strength * 100.0);

        // Update shape selection
        match marker_config.shape {
            MarkerShape::Circular => {
                imp.marker_shape_listbox.select_row(Some(
                    imp.shape_circular_row.upcast_ref::<gtk4::ListBoxRow>(),
                ));
            }
            MarkerShape::Rectangular => {
                imp.marker_shape_listbox.select_row(Some(
                    imp.shape_rectangular_row.upcast_ref::<gtk4::ListBoxRow>(),
                ));
            }
        }
    }
}
