use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::ops::Deref;
use std::time::Duration;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

pub struct PostgresConnection {
    _container: ContainerAsync<Postgres>,
    conn: DatabaseConnection,
}

impl Deref for PostgresConnection {
    type Target = DatabaseConnection;
    fn deref(&self) -> &Self::Target {
        return &self.conn;
    }
}

impl PostgresConnection {
    pub fn connection(&self) -> &DatabaseConnection {
        &self.conn
    }
}

pub async fn start_postgres() -> Result<PostgresConnection, String> {
    let container = Postgres::default()
        .start()
        .await
        .map_err(|e| format!("failed to start: {e}"))?;

    let host = container
        .get_host()
        .await
        .map_err(|e| format!("failed to get host: {e}"))?;
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .map_err(|e| format!("failed to get host post: {e}"))?;

    let conn_string =
        format!("postgres://postgres:postgres@{0}:{1}/postgres", host, port);

    let mut opt = ConnectOptions::new(conn_string);
    opt.min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false) // disable SQLx logging
        .sqlx_logging_level(log::LevelFilter::Info);

    let db_conn = Database::connect(opt)
        .await
        .map_err(|err| format!("db err {err}"))?;

    Ok(PostgresConnection {
        _container: container,
        conn: db_conn,
    })
}
