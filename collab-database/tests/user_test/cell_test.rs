use collab::core::any_map::AnyMapExtension;
use collab_database::rows::{new_cell_builder, CREATED_AT};
use collab_database::rows::{CreateRowParams, LAST_MODIFIED};
use collab_database::views::CreateDatabaseParams;

use crate::user_test::helper::{user_database_test, UserDatabaseTest};

#[test]
fn insert_cell_test() {
  let test = user_database_with_default_row();
  let database = test.get_database("d1").unwrap();
  database.update_row(&1.into(), |row_update| {
    row_update.update_cells(|cells_update| {
      cells_update.insert_cell(
        "f1",
        new_cell_builder(1).insert_i64_value("level", 1).build(),
      );
    });
  });

  let row = database.get_row(&1.into()).unwrap();
  let cell = row.cells.get("f1").unwrap();
  assert_eq!(cell.get_i64_value("level").unwrap(), 1);
}

#[test]
fn update_cell_test() {
  let test = user_database_with_default_row();
  let database = test.get_database("d1").unwrap();
  database.update_row(&1.into(), |row_update| {
    row_update.update_cells(|cells_update| {
      cells_update.insert_cell(
        "f1",
        new_cell_builder(1).insert_i64_value("level", 1).build(),
      );
    });
  });

  database.update_row(&1.into(), |row_update| {
    row_update.update_cells(|cells_update| {
      cells_update.insert(
        "f1",
        new_cell_builder(1)
          .insert_i64_value("level", 2)
          .insert_str_value("name", "appflowy")
          .build(),
      );
    });
  });

  let row = database.get_row(&1.into()).unwrap();
  let cell = row.cells.get("f1").unwrap();
  let created_at = cell.get_i64_value(CREATED_AT).unwrap();
  let modified_at = cell.get_i64_value(LAST_MODIFIED).unwrap();
  assert!(created_at > 0);
  assert!(modified_at > 0);
  assert_eq!(cell.get_i64_value("level").unwrap(), 2);
  assert_eq!(cell.get_str_value("name").unwrap(), "appflowy");
}

#[test]
fn update_not_exist_row_test() {
  let test = user_database_test(1);
  let database = test
    .create_database(CreateDatabaseParams {
      database_id: "d1".to_string(),
      view_id: "v1".to_string(),
      ..Default::default()
    })
    .unwrap();

  database.update_row(&1.into(), |_row_update| {});
  let row = database.get_row(&1.into());
  assert!(row.is_none())
}

fn user_database_with_default_row() -> UserDatabaseTest {
  let test = user_database_test(1);
  let database = test
    .create_database(CreateDatabaseParams {
      database_id: "d1".to_string(),
      view_id: "v1".to_string(),
      ..Default::default()
    })
    .unwrap();

  database.create_row_in_view(
    "v1",
    CreateRowParams {
      id: 1.into(),
      ..Default::default()
    },
  );

  test
}
