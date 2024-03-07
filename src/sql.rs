use anyhow::Context;
use async_std::net::TcpStream;
use tiberius::{Client, Config, ExecuteResult};
use tiberius::{Query, SqlBrowser};

/// Envoltura a la conexion con SQL Server
#[derive(Debug)]
pub struct SQL {
    pub client: Client<TcpStream>,
}

/// Estructura que define una fila de la tabla intermedia de la base de datos
#[derive(Debug)]
pub struct Etiquetas {
    pub id: u8,
    pub enable: bool,
    pub etiqueta: String,
    pub label: String,
    pub color: String,
}

impl SQL {
    /// Conexion con el SQL Server de runfo
    pub async fn new_connection() -> anyhow::Result<Self> {
        let mut config = Config::new();

        // Autenticacion de Windows
        config.authentication(tiberius::AuthMethod::Integrated);

        // SQL Server IP
        config.host("##<INSERRTAR IP>##");

        // Especificamos la base de datos para limitar su alcance en el servidor
        config.database("AuxiliarFaena");

        config.trust_cert();

        let tcp = TcpStream::connect_named(&config).await?;
        let client = Client::connect(config, tcp).await?;
        Ok(Self { client })
    }

    /// Ejecuta el Stored Procedure para cambiar las etiquetas
    pub async fn execute_cambiar_etiquetas(
        &mut self,
        ids: &str,
        etiqueta: &str,
    ) -> tiberius::Result<ExecuteResult> {
        self.client
            .execute(
                "EXECUTE [cambiarEtiquetas].[CambiarEtiquetas] @P1, @P2, @P3",
                &[&ids, &etiqueta, &"1"],
            )
            .await
    }

    /// Obtiene la tabla intermedia de etiquetas
    pub async fn query_table(&mut self) -> anyhow::Result<Vec<Etiquetas>> {
        let select = Query::new("SELECT * FROM [cambiarEtiquetas].[FaenaEtiquetas]");

        let stream = select.query(&mut self.client).await?;
        let rows = stream.into_results().await?;

        Ok(rows
            .get(0)
            .context("La query a la tabla 'FaenaEtiquetas' esta vacia.")?
            .iter()
            .map(|row| Etiquetas {
                id: row.get("id").expect("Columna 'id' no encontrada."),
                enable: row.get("enable").expect("Columna 'enable' no encontrada."),
                etiqueta: row
                    .get::<&str, &str>("etiqueta")
                    .expect("Columna 'etiqueta' no encontrada.")
                    .to_string(),
                label: row
                    .get::<&str, &str>("label")
                    .expect("Columna 'label' no encontrada.")
                    .to_string(),
                color: row
                    .get::<&str, &str>("color")
                    .expect("Columna 'color' no encontrada.")
                    .to_string(),
            })
            .collect())
    }

    /// Obtiene la lista de media reses.
    pub async fn query_ids(&mut self) -> anyhow::Result<String> {
        let select = Query::new(
            "DECLARE @mercaderias varchar(max)
            EXECUTE [cambiarEtiquetas].[ListarMedias] @mercaderias OUTPUT
            SELECT @mercaderias",
        );

        let stream = select.query(&mut self.client).await?;
        Ok(stream
            .into_row()
            .await?
            .context("La query 'ListarMedias' fallo.")?
            .get::<&str, usize>(0)
            .context("La query 'ListarMedias' esta vacia.")?
            .to_string())
    }
}

#[async_std::test]
async fn test_sql_connection_and_query_table() {
    let result = SQL::new_connection().await;
    assert!(result.is_ok());

    let result = result.unwrap().query_table().await;
    assert!(result.is_ok());
}

#[async_std::test]
async fn test_sql_connection_and_query_ids() {
    let result = SQL::new_connection().await;
    assert!(result.is_ok());

    let result = result.unwrap().query_ids().await;
    assert!(result.is_ok());
}
