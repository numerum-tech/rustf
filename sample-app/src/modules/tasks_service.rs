use crate::models::tasks::Tasks;
use chrono::Utc;
use rustf::models::BaseModel;
use std::borrow::Cow;
use std::collections::HashMap;

pub struct TasksService;

impl TasksService {
    pub async fn create(
        user_id: i64,
        list_id: i64,
        form: &HashMap<String, String>,
    ) -> rustf::Result<i64> {
        let title = required_field(form, "title")?;
        let details = optional_field(form, "details");

        let task = Tasks::builder()
            .title(title.as_ref())
            .details(details)
            .user_id(user_id as i32)
            .list_id(list_id as i32)
            .is_completed(0)
            .created_at(Utc::now().timestamp())
            .completed_at(None)
            .save()
            .await?;

        Ok(task.id() as i64)
    }

    pub async fn toggle(user_id: i64, task_id: i64) -> rustf::Result<i64> {
        let mut task = Self::find_owned(user_id, task_id).await?;
        let is_completed = task.is_completed() != 0;
        if is_completed {
            task.set_is_completed(0);
            task.set_completed_at(None);
        } else {
            task.set_is_completed(1);
            task.set_completed_at(Some(Utc::now().timestamp()));
        }
        let list_id = task.list_id() as i64;
        task.update().await?;
        Ok(list_id)
    }

    pub async fn delete(user_id: i64, task_id: i64) -> rustf::Result<i64> {
        let task = Self::find_owned(user_id, task_id).await?;
        let list_id = task.list_id() as i64;
        task.delete().await?;
        Ok(list_id)
    }

    async fn find_owned(user_id: i64, task_id: i64) -> rustf::Result<Tasks> {
        Tasks::query()?
            .where_eq("id", task_id)
            .where_eq("user_id", user_id)
            .get_first()
            .await?
            .ok_or_else(|| rustf::Error::validation("Task not found"))
    }
}

fn required_field<'a>(form: &'a HashMap<String, String>, key: &str) -> rustf::Result<Cow<'a, str>> {
    form.get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(Cow::from)
        .ok_or_else(|| rustf::Error::validation(format!("Field '{}' is required", key)))
}

fn optional_field(form: &HashMap<String, String>, key: &str) -> Option<String> {
    form.get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
