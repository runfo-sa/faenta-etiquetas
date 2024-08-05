mod toggle_switch;

use crate::{
    config::Config,
    constants,
    sql::{Etiquetas, SQL},
};
use async_std::task::block_on;
use egui::{Color32, Ui, Vec2};
use egui_modal::{Icon, Modal, ModalStyle};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use tiberius::{ExecuteResult, Result};
use toggle_switch::toggle;
use tracing::error;

/// Posibles estados de la aplicación
#[derive(Debug)]
enum AppStatus {
    Loading,
    Error,
    Warn,
    Ok,
}

#[derive(Debug)]
pub struct App {
    /// Cantidad de botones habilitados
    enables_count: u8,
    /// Lista de mercaderia separada por comas, ejemplo: "11,12,13,14"
    faena_ids: String,
    /// Estado de conexion de la aplicación
    status: AppStatus,
    /// Tabla intermedia con informacion sobre cada etiqueta disponible
    table: Option<Vec<Etiquetas>>,
    /// Conexion con el servidor de SQL
    sql_client: Option<Arc<Mutex<SQL>>>,
    /// Hilo secundario para ejecutar las llamadas al servidor
    handler: Option<JoinHandle<Result<ExecuteResult>>>,
    /// Configuraciones del programa
    config: Config,
}

impl App {
    pub async fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config: Config = confy::load("faena_etiquetas", "config")
            .expect("No se pudo generar el archivo de configuracion.");
        let sql_client = SQL::new_connection().await;

        if let Ok(mut sql) = sql_client {
            let (table, enables_count) = App::update_table(&mut sql, config.is_dpi300).await;
            let faena_ids = sql.query_ids().await;

            Self {
                table,
                enables_count,
                handler: None,
                status: AppStatus::Error,
                sql_client: Some(Arc::new(Mutex::new(sql))),
                faena_ids: if let Err(err) = faena_ids {
                    error!("On sql::query_ids: {err}");
                    String::new()
                } else {
                    faena_ids.unwrap()
                },
                config,
            }
        } else {
            let err = sql_client.as_ref().err().unwrap();
            error!("On sql::new_connection: {err}");

            Self {
                enables_count: 0,
                faena_ids: String::new(),
                status: AppStatus::Error,
                table: None,
                sql_client: None,
                handler: None,
                config,
            }
        }
    }

    async fn update_table(sql_client: &mut SQL, is_300dpi: bool) -> (Option<Vec<Etiquetas>>, u8) {
        let table = sql_client.query_table(is_300dpi).await;
        if let Err(err) = table.as_ref() {
            error!("On sql::query_table: {err}");
            return (None, 0);
        }

        let table = table.unwrap();
        let value = table.iter().filter(|e| e.enable).count() as u8;

        (Some(table), value)
    }

    #[inline]
    /// Arma la grilla con todos los botones a mostrar
    fn build_grid(&mut self, ui: &mut Ui, modal: &Modal) {
        // Fuente mas grande para los botones.
        ui.style_mut().text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(
                constants::BUTTON_FONT_SIZE,
                eframe::epaint::FontFamily::Proportional,
            ),
        );

        egui::Grid::new("faena_grid")
            .spacing(egui::Vec2::new(
                constants::GRID_SPACE,
                constants::GRID_SPACE,
            ))
            .show(ui, |ui| {
                if self.table.is_none() {
                    return;
                }

                // Itera sobre los botones habilitados
                let table = self.table.as_ref().unwrap().iter().filter(|eti| eti.enable);
                for (i, eti) in table.enumerate() {
                    // Debug! to remove. Para mostrar menos botones de los habilitados.
                    #[cfg(debug_assertions)]
                    if i >= self.enables_count as usize {
                        break;
                    }

                    if ui
                        .add_enabled(
                            // Deshabilita el boton en caso de fallar la conexion con SQL Server.
                            self.sql_client.is_some(),
                            egui::Button::new(egui::RichText::new(&eti.label).strong())
                                .fill(Color32::from_hex(&eti.color).unwrap())
                                .min_size(Vec2::new(
                                    constants::BUTTON_WIDTH,
                                    constants::BUTTON_HEIGHT,
                                )),
                        )
                        .clicked()
                    {
                        // Ejecuta la SP para cambiar la etiqueta.
                        if let Some(sql) = &mut self.sql_client {
                            let sql = sql.clone();
                            let eti = eti.etiqueta.clone();
                            let ids = self.faena_ids.clone();

                            // Pasamos la ejecucion de la query a otro hilo para no trabar la interfaz.
                            self.status = AppStatus::Loading;
                            self.handler = Some(std::thread::spawn(move || {
                                block_on(sql.lock().unwrap().execute_cambiar_etiquetas(&ids, &eti))
                            }));
                        }

                        modal.open()
                    }

                    // Cada 3 botones salta de fila
                    if (i + 1) % 3 == 0 {
                        ui.end_row()
                    }
                }
            });
    }

    fn refresh_table(&mut self) {
        if let Some(sql) = &mut self.sql_client {
            (self.table, self.enables_count) = block_on(App::update_table(
                sql.clone().lock().as_mut().unwrap(),
                self.config.is_dpi300,
            ))
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Analizamos si el hilo secundario esta corriendo (is_some) y si ya termino con su tarea
        if self.handler.as_ref().is_some_and(|task| task.is_finished()) {
            let rc = self.handler.take().unwrap().join().unwrap();

            if let Err(err) = rc {
                if err.code().is_some_and(|code| code == constants::WARN_CODE) {
                    // Actualizamos la tabla intermedia
                    self.refresh_table();
                    self.status = AppStatus::Warn
                } else {
                    self.status = AppStatus::Error;
                    error!("Fallo el cambio de etiqueta. Motivo: {err}")
                }
            } else {
                // Actualizamos la tabla intermedia
                self.refresh_table();
                self.status = AppStatus::Ok
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| ui.label("RUNFO S.A."));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Titulo, avisa si esta desconectado.
                if self.sql_client.is_none() {
                    ui.heading(
                        egui::RichText::new("desconectado")
                            .color(Color32::RED)
                            .strong(),
                    );
                } else {
                    ui.heading("etiquetas faena");
                }

                if ui.add(egui::Button::new("⟳")).clicked() {
                    // Actualizamos la tabla intermedia
                    self.refresh_table()
                }
            });

            // Debug! to remove.
            #[cfg(debug_assertions)]
            ui.add(
                egui::Slider::new(
                    &mut self.enables_count,
                    constants::MIN_ETI..=constants::MAX_ETI,
                )
                .text("(Debug!) Etiquetas"),
            );

            // Dpi switch
            ui.horizontal(|ui| {
                if ui.add(toggle(&mut self.config.is_dpi300)).changed() {
                    self.refresh_table();
                    if let Err(error) = confy::store("faena_etiquetas", "config", &self.config) {
                        error!("No se pudo guardar la configuracion debido a: {:#?}", error)
                    }
                }
                ui.heading(if self.config.is_dpi300 {
                    "300 dpi"
                } else {
                    "203 dpi"
                })
                .highlight();
            });

            // Espaciado vertical inteligente.
            ui.add_space(
                (ui.available_height()
                    - constants::BUTTON_HEIGHT * f32::ceil(self.enables_count as f32 / 3.0)
                    - constants::GRID_SPACE * f32::floor(self.enables_count as f32 / 3.0).min(1.0))
                    / 2.0,
            );

            // Scrollbar en caso de que la lista de botones se vaya de la pantalla, almenos todavia podrian ser accesibles.
            egui::ScrollArea::both().show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        // Espaciado horizontal inteligente.
                        ui.add_space(
                            (ui.available_width()
                                - constants::BUTTON_WIDTH * self.enables_count.min(3) as f32
                                - constants::GRID_SPACE
                                    * f32::ceil(self.enables_count as f32 / 2.0).min(2.0))
                                / 2.0,
                        );

                        let modal = Modal::new(ctx, "confirmation_modal").with_style(&ModalStyle {
                            default_height: Some(constants::MODAL_HEIGHT),
                            default_width: Some(constants::MODAL_WIDTH),
                            body_alignment: egui::Align::Center,
                            icon_size: constants::ICON_SIZE,
                            ..Default::default()
                        });

                        modal.show(|ui| {
                            modal.title(ui, "Cambiando etiquetas...");

                            modal.frame(ui, |ui| {
                                ui.style_mut().text_styles.insert(
                                    egui::TextStyle::Body,
                                    egui::FontId::new(
                                        32.0,
                                        eframe::epaint::FontFamily::Proportional,
                                    ),
                                );

                                match self.status {
                                    AppStatus::Ok => modal.body_and_icon(
                                        ui,
                                        "Etiquetas cambiadas exitosamente",
                                        Icon::Success,
                                    ),
                                    AppStatus::Error => {
                                        modal.body_and_icon(ui, constants::ERROR_MSG, Icon::Error)
                                    }
                                    AppStatus::Warn => {
                                        modal.body_and_icon(ui, constants::WARN_MSG, Icon::Warning)
                                    }
                                    AppStatus::Loading => {
                                        ui.add(egui::Spinner::new());
                                    }
                                }
                            });

                            modal.buttons(ui, |ui| {
                                if let AppStatus::Loading = self.status {
                                    return;
                                }

                                ui.style_mut().text_styles.insert(
                                    egui::TextStyle::Button,
                                    egui::FontId::new(
                                        24.0,
                                        eframe::epaint::FontFamily::Proportional,
                                    ),
                                );

                                // Tamaño del boton "Okay", calculado manualmente.
                                // Hardcodeado porque es imposible de saber en esta parte.
                                ui.add_space((ui.available_width() - 70.41656) / 2.0);

                                if modal.button(ui, "Okay").clicked() {
                                    self.status = AppStatus::Error;
                                }
                            });
                        });

                        self.build_grid(ui, &modal);
                    },
                );
            });
        });
    }
}
