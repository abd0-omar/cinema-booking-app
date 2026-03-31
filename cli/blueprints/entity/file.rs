#[cfg(feature = "test-helpers")]
use fake::{faker::lorem::en::*, Dummy};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Sqlite;
use validator::Validate;

#[derive(Serialize, Debug, Deserialize)]
pub struct {{entity_struct_name}} {
    pub id: i64,
    pub uuid: String,
    {%- for field in fields %}
    pub {{ field.name }}: {{ field.type }},
    {%- endfor %}
}

#[derive(Deserialize, Validate, Clone)]
#[cfg_attr(feature = "test-helpers", derive(Serialize, Dummy))]
pub struct {{entity_struct_name}}Changeset {
    {%- for field in fields %}
    //#[cfg_attr(feature = "test-helpers", dummy(faker = "…()"))]
    //#[validate(…))]
    pub {{ field.name }}: {{ field.type }},
    {%- endfor %}
}

pub async fn load_all(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Vec<{{entity_struct_name}}>, crate::Error> {
    let {{entity_plural_name}} = sqlx::query_as!({{entity_struct_name}}, "SELECT id, uuid, {{ fields | map: \"name\" | join: \", \" }} FROM {{entity_plural_name}}")
        .fetch_all(executor)
        .await?;
    Ok({{entity_plural_name}})
}

pub async fn load(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<{{entity_struct_name}}, crate::Error> {
    match sqlx::query_as!(
        {{entity_struct_name}},
        "SELECT id, uuid, {{ fields | map: \"name\" | join: \", \" }} FROM {{entity_plural_name}} WHERE uuid = ?1",
        uuid
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    {
        Some({{entity_singular_name}}) => Ok({{entity_singular_name}}),
        None => Err(crate::Error::NoRecordFound),
    }
}

pub async fn create(
    {{entity_singular_name}}: {{entity_struct_name}}Changeset,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<{{entity_struct_name}}, crate::Error> {
    {{entity_singular_name}}.validate()?;

    let uuid = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query!(
        "INSERT INTO {{entity_plural_name}} (uuid, {{ fields | map: \"name\" | join: \", \" }}) VALUES (?1, {%- for field in fields -%}?{{ forloop.index | plus: 1 }}{%- unless forloop.last -%}, {% endunless -%}{%- endfor -%})",
        uuid,
        {%- for field in fields %}
        {{entity_singular_name}}.{{ field.name }},
        {%- endfor %}
    )
    .execute(executor)
    .await
    .map_err(crate::Error::DbError)?;

    Ok({{entity_struct_name}} {
        id: result.last_insert_rowid(),
        uuid,
        {%- for field in fields %}
        {{ field.name }}: {{entity_singular_name}}.{{ field.name }},
        {%- endfor %}
    })
}

pub async fn update(
    uuid: &str,
    {{entity_singular_name}}: {{entity_struct_name}}Changeset,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<{{entity_struct_name}}, crate::Error> {
    {{entity_singular_name}}.validate()?;

    let result = sqlx::query!(
        "UPDATE {{entity_plural_name}} SET {% for field in fields -%}{{ field.name }} = ?{{ forloop.index }}{%- unless forloop.last -%}, {% endunless -%}{%- endfor %} WHERE uuid = ?{{ fields | size | plus: 1 }}",
        {%- for field in fields %}
        {{entity_singular_name}}.{{ field.name }},
        {%- endfor %}
        uuid
    )
    .execute(executor)
    .await
    .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    match sqlx::query_as!(
        {{entity_struct_name}},
        "SELECT id, uuid, {{ fields | map: \"name\" | join: \", \" }} FROM {{entity_plural_name}} WHERE uuid = ?1",
        uuid
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    {
        Some(record) => Ok(record),
        None => Err(crate::Error::NoRecordFound),
    }
}

pub async fn delete(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<(), crate::Error> {
    let result = sqlx::query!("DELETE FROM {{entity_plural_name}} WHERE uuid = ?1", uuid)
        .execute(executor)
        .await
        .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    Ok(())
}
