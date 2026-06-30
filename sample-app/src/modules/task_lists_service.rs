use crate::models::task_lists::TaskLists;
use crate::models::tasks::Tasks;
use rustf::models::BaseModel;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;

pub struct TaskListsService;

#[derive(Debug, Clone, Serialize)]
pub struct TaskListView {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub tasks: Vec<TaskView>,
    pub total_count: usize,
    pub completed_count: usize,
    pub pending_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub id: i64,
    pub list_id: i64,
    pub title: String,
    pub details: Option<String>,
    pub is_completed: bool,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

impl TaskListsService {
    pub async fn dashboard(user_id: i64) -> rustf::Result<Vec<TaskListView>> {
        let lists = TaskLists::query()?
            .where_eq("user_id", user_id)
            .get_all()
            .await?;

        let mut output = Vec::with_capacity(lists.len());
        for list in lists {
            output.push(Self::build_list_view(list, user_id).await?);
        }

        Ok(output)
    }

    pub async fn list_fragment(user_id: i64, list_id: i64) -> rustf::Result<TaskListView> {
        let list = Self::find_owned(user_id, list_id).await?;
        Self::build_list_view(list, user_id).await
    }

    pub async fn find_owned(user_id: i64, id: i64) -> rustf::Result<TaskLists> {
        let Some(item) = Self::find(user_id, id).await? else {
            return Err(rustf::Error::validation("Task list not found"));
        };
        Ok(item)
    }

    pub async fn find(user_id: i64, id: i64) -> rustf::Result<Option<TaskLists>> {
        TaskLists::query()?
            .where_eq("id", id)
            .where_eq("user_id", user_id)
            .get_first()
            .await
    }

    pub async fn create(user_id: i64, form: &HashMap<String, String>) -> rustf::Result<i64> {
        let title = required_field(form, "title")?;
        let description = optional_field(form, "description");

        let item = TaskLists::builder()
            .title(title.as_ref())
            .description(description)
            .user_id(user_id as i32)
            .created_at(chrono::Utc::now().timestamp())
            .save()
            .await?;

        Ok(item.id() as i64)
    }

    pub async fn update(
        user_id: i64,
        id: i64,
        form: &HashMap<String, String>,
    ) -> rustf::Result<()> {
        let mut item = Self::find_owned(user_id, id).await?;
        item.set_title(required_field(form, "title")?.as_ref());
        item.set_description(optional_field(form, "description"));
        item.update().await
    }

    pub async fn delete(user_id: i64, id: i64) -> rustf::Result<()> {
        let item = Self::find_owned(user_id, id).await?;
        let tasks = Tasks::query()?
            .where_eq("user_id", user_id)
            .where_eq("list_id", id)
            .get_all()
            .await?;
        for task in tasks {
            task.delete().await?;
        }
        item.delete().await
    }

    async fn build_list_view(list: TaskLists, user_id: i64) -> rustf::Result<TaskListView> {
        let tasks = Tasks::query()?
            .where_eq("user_id", user_id)
            .where_eq("list_id", list.id() as i64)
            .get_all()
            .await?;

        let task_views = tasks
            .into_iter()
            .map(|task| TaskView {
                id: task.id() as i64,
                list_id: task.list_id() as i64,
                title: task.title().to_string(),
                details: task.details().map(str::to_string),
                is_completed: task.is_completed() != 0,
                created_at: task.created_at(),
                completed_at: task.completed_at(),
            })
            .collect::<Vec<_>>();

        let total_count = task_views.len();
        let completed_count = task_views.iter().filter(|task| task.is_completed).count();
        let pending_count = total_count.saturating_sub(completed_count);

        Ok(TaskListView {
            id: list.id() as i64,
            title: list.title().to_string(),
            description: list.description().map(str::to_string),
            created_at: list.created_at(),
            tasks: task_views,
            total_count,
            completed_count,
            pending_count,
        })
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
