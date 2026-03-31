use crate::entities::{{entity_plural_name}}::{{entity_struct_name}};
use fake::{faker::name::en::*, Dummy};
use sqlx::SqlitePool;
use validator::Validate;

#[derive(Debug, Clone, Dummy, Validate)]
pub struct {{entity_struct_name}}Changeset {
    // these are examples only
    #[dummy(faker = "Name()")]
    #[validate(length(min = 1))]
    pub name: String,
}

pub async fn create({{entity_singular_name}}: {{entity_struct_name}}Changeset, db: &SqlitePool) -> Result<{{entity_struct_name}}, anyhow::Error> {
    todo!("Adopt the SQL query as necessary!");
    let uuid = uuid::Uuid::new_v4().to_string();
    
    let result = sqlx::query!(
        "INSERT INTO {{entity_plural_name}} (uuid, name) VALUES (?1, ?2)",
        uuid,
        {{entity_singular_name}}.name,
    )
    .execute(db)
    .await?;

    Ok({{entity_struct_name}} { 
        id: result.last_insert_rowid(),
        uuid,
        name: {{entity_singular_name}}.name 
    })
}
