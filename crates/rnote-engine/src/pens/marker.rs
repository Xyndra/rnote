// Imports
use super::PenBehaviour;
use super::PenStyle;
use crate::engine::{EngineView, EngineViewMut};
use crate::store::StrokeKey;
use crate::strokes::MarkerStroke;
use crate::strokes::Stroke;
use crate::{DrawableOnDoc, WidgetFlags};
use p2d::bounding_volume::{Aabb, BoundingVolume};
use rnote_compose::Constraints;
use rnote_compose::builders::PenPathSimpleBuilder;
use rnote_compose::builders::buildable::{Buildable, BuilderCreator, BuilderProgress};
use rnote_compose::eventresult::{EventPropagation, EventResult};
use rnote_compose::penevent::{PenEvent, PenProgress};
use rnote_compose::penpath::Segment;
use std::time::Instant;

#[derive(Debug)]
enum MarkerState {
    Idle,
    Drawing {
        path_builder: Box<dyn Buildable<Emit = Segment>>,
        current_stroke_key: StrokeKey,
    },
}

#[derive(Debug)]
pub struct Marker {
    state: MarkerState,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            state: MarkerState::Idle,
        }
    }
}

impl PenBehaviour for Marker {
    fn init(&mut self, _engine_view: &EngineView) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn deinit(&mut self) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn style(&self) -> PenStyle {
        PenStyle::Marker
    }

    fn update_state(&mut self, _engine_view: &mut EngineViewMut) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn handle_event(
        &mut self,
        event: PenEvent,
        now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (EventResult<PenProgress>, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();

        let event_result = match (&mut self.state, event) {
            (MarkerState::Idle, PenEvent::Down { element, .. }) => {
                if !element.filter_by_bounds(
                    engine_view
                        .document
                        .bounds()
                        .loosened(Self::INPUT_OVERSHOOT),
                ) {
                    let marker_config = &engine_view.config.pens_config.marker_config;

                    let markerstroke = Stroke::MarkerStroke(MarkerStroke::new(
                        element,
                        marker_config.width,
                        marker_config.shape,
                        marker_config.effective_color(),
                    ));

                    let current_stroke_key = engine_view
                        .store
                        .insert_stroke(markerstroke, Some(marker_config.layer()));

                    engine_view.store.regenerate_rendering_for_stroke(
                        current_stroke_key,
                        engine_view.camera.viewport(),
                        engine_view.camera.image_scale(),
                    );

                    self.state = MarkerState::Drawing {
                        path_builder: new_builder(element, now),
                        current_stroke_key,
                    };

                    EventResult {
                        handled: true,
                        propagate: EventPropagation::Stop,
                        progress: PenProgress::InProgress,
                    }
                } else {
                    EventResult {
                        handled: false,
                        propagate: EventPropagation::Proceed,
                        progress: PenProgress::Idle,
                    }
                }
            }
            (MarkerState::Idle, _) => EventResult {
                handled: false,
                propagate: EventPropagation::Proceed,
                progress: PenProgress::Idle,
            },
            (
                MarkerState::Drawing {
                    current_stroke_key, ..
                },
                PenEvent::Cancel,
            ) => {
                // Finish up the last stroke
                engine_view
                    .store
                    .update_geometry_for_stroke(*current_stroke_key);
                engine_view.store.regenerate_rendering_for_stroke_threaded(
                    engine_view.tasks_tx.clone(),
                    *current_stroke_key,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );
                widget_flags |= engine_view
                    .document
                    .resize_autoexpand(engine_view.store, engine_view.camera);

                self.state = MarkerState::Idle;

                widget_flags |= engine_view.store.record(Instant::now());
                widget_flags.store_modified = true;

                EventResult {
                    handled: true,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::Finished,
                }
            }
            (
                MarkerState::Drawing {
                    path_builder,
                    current_stroke_key,
                },
                pen_event,
            ) => {
                let builder_result =
                    path_builder.handle_event(pen_event, now, Constraints::default());
                let handled = builder_result.handled;
                let propagate = builder_result.propagate;

                let progress = match builder_result.progress {
                    BuilderProgress::InProgress => PenProgress::InProgress,
                    BuilderProgress::EmitContinue(segments) => {
                        let n_segments = segments.len();

                        if n_segments != 0 {
                            if let Some(Stroke::MarkerStroke(markerstroke)) =
                                engine_view.store.get_stroke_mut(*current_stroke_key)
                            {
                                markerstroke.extend_w_segments(segments);
                                widget_flags.store_modified = true;
                            }

                            // Use incremental rendering during drawing for performance
                            engine_view.store.append_rendering_last_segments(
                                engine_view.tasks_tx.clone(),
                                *current_stroke_key,
                                n_segments,
                                engine_view.camera.viewport(),
                                engine_view.camera.image_scale(),
                            );
                        }

                        PenProgress::InProgress
                    }
                    BuilderProgress::Finished(segments) => {
                        let n_segments = segments.len();

                        if n_segments != 0 {
                            if let Some(Stroke::MarkerStroke(markerstroke)) =
                                engine_view.store.get_stroke_mut(*current_stroke_key)
                            {
                                markerstroke.extend_w_segments(segments);
                                widget_flags.store_modified = true;
                            }

                            engine_view.store.append_rendering_last_segments(
                                engine_view.tasks_tx.clone(),
                                *current_stroke_key,
                                n_segments,
                                engine_view.camera.viewport(),
                                engine_view.camera.image_scale(),
                            );
                        }

                        // Finish up the stroke - regenerate the entire stroke to avoid self-overlap
                        engine_view
                            .store
                            .update_geometry_for_stroke(*current_stroke_key);
                        engine_view.store.regenerate_rendering_for_stroke_threaded(
                            engine_view.tasks_tx.clone(),
                            *current_stroke_key,
                            engine_view.camera.viewport(),
                            engine_view.camera.image_scale(),
                        );

                        widget_flags |= engine_view
                            .document
                            .resize_autoexpand(engine_view.store, engine_view.camera);

                        self.state = MarkerState::Idle;

                        widget_flags |= engine_view.store.record(Instant::now());
                        widget_flags.store_modified = true;

                        PenProgress::Finished
                    }
                };

                EventResult {
                    handled,
                    propagate,
                    progress,
                }
            }
        };

        (event_result, widget_flags)
    }

    fn handle_animation_frame(&mut self, _engine_view: &mut EngineViewMut) {}

    fn fetch_clipboard_content(
        &self,
        _engine_view: &EngineView,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<(Vec<(Vec<u8>, String)>, WidgetFlags)>>
    {
        let (sender, receiver) = futures::channel::oneshot::channel();
        if sender.send(Ok((vec![], WidgetFlags::default()))).is_err() {
            tracing::error!("Failed to send clipboard content for marker");
        }
        receiver
    }

    fn cut_clipboard_content(
        &mut self,
        _engine_view: &mut EngineViewMut,
    ) -> futures::channel::oneshot::Receiver<anyhow::Result<(Vec<(Vec<u8>, String)>, WidgetFlags)>>
    {
        let (sender, receiver) = futures::channel::oneshot::channel();
        if sender.send(Ok((vec![], WidgetFlags::default()))).is_err() {
            tracing::error!("Failed to send clipboard content for marker");
        }
        receiver
    }
}

impl DrawableOnDoc for Marker {
    fn bounds_on_doc(&self, _engine_view: &EngineView) -> Option<Aabb> {
        None
    }

    fn draw_on_doc(
        &self,
        _cx: &mut piet_cairo::CairoRenderContext,
        _engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Marker {
    const INPUT_OVERSHOOT: f64 = 30.0;
}

fn new_builder(
    element: rnote_compose::penpath::Element,
    now: Instant,
) -> Box<dyn Buildable<Emit = Segment>> {
    // Use simple builder for markers - no pressure sensitivity, just clean lines
    Box::new(PenPathSimpleBuilder::start(element, now))
}
