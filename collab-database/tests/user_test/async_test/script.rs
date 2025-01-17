#![allow(clippy::all)]

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use collab_database::database::DatabaseData;
use collab_database::fields::Field;
use collab_database::rows::CreateRowParams;
use collab_database::rows::{Cells, CellsBuilder, RowId};
use collab_database::user::UserDatabase as InnerUserDatabase;
use collab_database::views::CreateDatabaseParams;
use collab_persistence::doc::YrsDocAction;
use collab_persistence::kv::rocks_kv::RocksCollabDB;
use collab_plugins::disk::rocksdb::CollabPersistenceConfig;
use parking_lot::Mutex;
use serde_json::Value;

use crate::helper::{db_path, TestTextCell};
use crate::user_test::helper::UserDatabaseCollabBuilderImpl;

pub enum DatabaseScript {
  CreateDatabase {
    params: CreateDatabaseParams,
  },
  OpenDatabase {
    database_id: String,
  },
  CloseDatabase {
    database_id: String,
  },
  CreateRow {
    database_id: String,
    params: CreateRowParams,
  },
  EditRow {
    database_id: String,
    row_id: RowId,
    cells: Cells,
  },
  AssertDatabaseInDisk {
    database_id: String,
    expected: Value,
  },
  AssertDatabase {
    database_id: String,
    expected: Value,
  },
  AssertNumOfUpdates {
    oid: String,
    expected: usize,
  },
  IsExist {
    oid: String,
    expected: bool,
  },
}

#[derive(Clone)]
pub struct DatabaseTest {
  pub db: Arc<RocksCollabDB>,
  pub db_path: PathBuf,
  pub user_database: UserDatabase,
  pub config: CollabPersistenceConfig,
}

pub fn database_test(config: CollabPersistenceConfig) -> DatabaseTest {
  DatabaseTest::new(config)
}

impl DatabaseTest {
  pub fn new(config: CollabPersistenceConfig) -> Self {
    let db_path = db_path();
    let db = Arc::new(RocksCollabDB::open(db_path.clone()).unwrap());
    let collab_builder = UserDatabaseCollabBuilderImpl();
    let inner = InnerUserDatabase::new(1, db.clone(), config.clone(), collab_builder);
    let user_database = UserDatabase(Arc::new(Mutex::new(inner)));
    Self {
      db,
      user_database,
      db_path,
      config,
    }
  }

  #[allow(dead_code)]
  pub fn get_database_data(&self, database_id: &str) -> DatabaseData {
    let database = self.user_database.lock().get_database(database_id).unwrap();
    database.duplicate_database()
  }

  pub async fn run_scripts(&mut self, scripts: Vec<DatabaseScript>) {
    let mut handles = vec![];
    for script in scripts {
      let user_database = self.user_database.clone();
      let db = self.db.clone();
      let config = self.config.clone();
      let handle = tokio::spawn(async move {
        run_script(user_database, db, config, script);
      });
      handles.push(handle);
    }
    for result in futures::future::join_all(handles).await {
      assert!(result.is_ok());
    }
  }
}

pub fn run_script(
  user_database: UserDatabase,
  db: Arc<RocksCollabDB>,
  config: CollabPersistenceConfig,
  script: DatabaseScript,
) {
  match script {
    DatabaseScript::CreateDatabase { params } => {
      user_database.lock().create_database(params).unwrap();
    },
    DatabaseScript::OpenDatabase { database_id } => {
      user_database.lock().get_database(&database_id).unwrap();
    },
    DatabaseScript::CloseDatabase { database_id } => {
      user_database.lock().close_database(&database_id);
    },
    DatabaseScript::CreateRow {
      database_id,
      params,
    } => {
      user_database
        .lock()
        .get_database(&database_id)
        .unwrap()
        .create_row(params)
        .unwrap();
    },
    DatabaseScript::EditRow {
      database_id,
      row_id,
      cells,
    } => {
      user_database
        .lock()
        .get_database(&database_id)
        .unwrap()
        .update_row(&row_id, |row| {
          row.set_cells(cells);
        });
    },
    // DatabaseScript::CreateField { database_id, field } => {
    //   user_database
    //     .lock()
    //     .get_database(&database_id)
    //     .unwrap()
    //     .create_field(field);
    // },
    DatabaseScript::AssertDatabaseInDisk {
      database_id,
      expected,
    } => {
      let collab_builder = UserDatabaseCollabBuilderImpl();
      let inner = InnerUserDatabase::new(1, db, config, collab_builder);
      let database = inner.get_database(&database_id).unwrap();
      let actual = database.to_json_value();
      assert_json_diff::assert_json_include!(actual: actual, expected: expected);
    },
    DatabaseScript::AssertDatabase {
      database_id,
      expected,
    } => {
      let database = user_database.lock().get_database(&database_id).unwrap();
      let actual = database.to_json_value();
      assert_json_diff::assert_json_include!(actual: actual, expected: expected);
    },
    DatabaseScript::IsExist {
      oid: database_id,
      expected,
    } => {
      assert_eq!(db.read_txn().is_exist(1, &database_id), expected,)
    },
    DatabaseScript::AssertNumOfUpdates {
      oid: database_id,
      expected,
    } => {
      let updates = db
        .read_txn()
        .get_decoded_v1_updates(1, &database_id)
        .unwrap();
      assert_eq!(updates.len(), expected,);
    },
  }
}

pub fn create_database(database_id: &str) -> CreateDatabaseParams {
  let row_1 = CreateRowParams {
    id: 1.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("1f1cell"))
      .insert_cell("f2", TestTextCell::from("1f2cell"))
      .insert_cell("f3", TestTextCell::from("1f3cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let row_2 = CreateRowParams {
    id: 2.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("2f1cell"))
      .insert_cell("f2", TestTextCell::from("2f2cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let row_3 = CreateRowParams {
    id: 3.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("3f1cell"))
      .insert_cell("f3", TestTextCell::from("3f3cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let field_1 = Field::new("f1".to_string(), "text field".to_string(), 0, true);
  let field_2 = Field::new("f2".to_string(), "single select field".to_string(), 2, true);
  let field_3 = Field::new("f3".to_string(), "checkbox field".to_string(), 1, true);

  CreateDatabaseParams {
    database_id: database_id.to_string(),
    view_id: "v1".to_string(),
    name: "my first database".to_string(),
    layout: Default::default(),
    layout_settings: Default::default(),
    filters: vec![],
    groups: vec![],
    sorts: vec![],
    created_rows: vec![row_1, row_2, row_3],
    fields: vec![field_1, field_2, field_3],
  }
}

#[derive(Clone)]
pub struct UserDatabase(Arc<Mutex<InnerUserDatabase>>);

impl Deref for UserDatabase {
  type Target = Arc<Mutex<InnerUserDatabase>>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

unsafe impl Sync for UserDatabase {}

unsafe impl Send for UserDatabase {}
