use chrono::{DateTime, Local, NaiveDateTime};
use clap::{Parser, Subcommand};
use colored::*;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Category {
    Personal,
    Work,
    Shopping,
    Health,
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: usize,
    title: String,
    description: Option<String>,
    completed: bool,
    created_at: DateTime<Local>,
    due_date: Option<NaiveDateTime>,
    priority: Priority,
    category: Category,
    tags: Vec<String>,
}

#[derive(Parser)]
#[command(
    name = "todo",
    about = "A feature-rich CLI todo manager",
    version = "0.2.0",
    author = "Ali Mert"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {

        title: String,

        #[arg(short, long)]
        description: Option<String>,

        #[arg(short, long)]
        due: Option<String>,

        #[arg(short, long, default_value = "medium")]
        priority: String,

        #[arg(short, long, default_value = "personal")]
        category: String,

        #[arg(short, long)]
        tags: Option<String>,
    },

    List {

        #[arg(short, long)]
        category: Option<String>,

        #[arg(short, long)]
        priority: Option<String>,

        #[arg(short, long)]
        completed: bool,

        #[arg(short, long)]
        pending: bool,
    },

    Show {

        id: usize,
    },

    Complete {

        id: usize,
    },
    Remove {
        id: usize,
    },
    Edit {

        id: usize,

        #[arg(short, long)]
        title: Option<String>,

        #[arg(short, long)]
        description: Option<String>,

        #[arg(short, long)]
        due: Option<String>,

        #[arg(short, long)]
        priority: Option<String>,

        #[arg(short, long)]
        category: Option<String>,

        #[arg(short, long)]
        tags: Option<String>,
    },
}

impl Task {
    fn new(
        id: usize,
        title: String,
        description: Option<String>,
        due_date: Option<String>,
        priority: &str,
        category: &str,
        tags: Option<String>,
    ) -> Result<Self, String> {
        let priority = match priority.to_lowercase().as_str() {
            "low" => Priority::Low,
            "medium" => Priority::Medium,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            _ => return Err("Invalid priority level".to_string()),
        };

        let category = match category.to_lowercase().as_str() {
            "personal" => Category::Personal,
            "work" => Category::Work,
            "shopping" => Category::Shopping,
            "health" => Category::Health,
            other => Category::Other(other.to_string()),
        };

        let due_date = if let Some(date_str) = due_date {
            Some(
                NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M")
                    .map_err(|e| format!("Invalid date format: {}", e))?,
            )
        } else {
            None
        };

        let tags = tags
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Task {
            id,
            title,
            description,
            completed: false,
            created_at: Local::now(),
            due_date,
            priority,
            category,
            tags,
        })
    }

    fn to_row(&self) -> Row {
        let status = if self.completed {
            "✓".green()
        } else {
            "✗".red()
        };

        let priority_color = match self.priority {
            Priority::Low => "Low".normal(),
            Priority::Medium => "Medium".yellow(),
            Priority::High => "High".bright_red(),
            Priority::Critical => "Critical".red().bold(),
        };

        let category_str = match &self.category {
            Category::Personal => "Personal",
            Category::Work => "Work",
            Category::Shopping => "Shopping",
            Category::Health => "Health",
            Category::Other(s) => s,
        };

        let due_date = self
            .due_date
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        Row::new(vec![
            Cell::new(&self.id.to_string()),
            Cell::new(&status.to_string()),
            Cell::new(&self.title),
            Cell::new(&due_date),
            Cell::new(&priority_color.to_string()),
            Cell::new(category_str),
            Cell::new(&self.tags.join(", ")),
        ])
    }
}

struct TodoManager {
    tasks: Vec<Task>,
    file_path: PathBuf,
}

impl TodoManager {
    fn new() -> Self {
        let file_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".todo-cli.json");
        
        let tasks = if file_path.exists() {
            let contents = fs::read_to_string(&file_path).unwrap_or_else(|_| "[]".to_string());
            serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        TodoManager { tasks, file_path }
    }

    fn save(&self) -> Result<(), String> {
        let contents = serde_json::to_string_pretty(&self.tasks)
            .map_err(|e| format!("Failed to serialize tasks: {}", e))?;
        fs::write(&self.file_path, contents)
            .map_err(|e| format!("Failed to save tasks: {}", e))?;
        Ok(())
    }

    fn add_task(&mut self, task: Task) -> Result<(), String> {
        self.tasks.push(task);
        self.save()?;
        Ok(())
    }

    fn list_tasks(&self, filters: &ListFilters) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| {
                let category_match = filters
                    .category
                    .as_ref()
                    .map(|c| match &task.category {
                        Category::Other(s) => s == c,
                        _ => c == &format!("{:?}", task.category).to_lowercase(),
                    })
                    .unwrap_or(true);

                let priority_match = filters
                    .priority
                    .as_ref()
                    .map(|p| format!("{:?}", task.priority).to_lowercase() == *p)
                    .unwrap_or(true);

                let completion_match = if filters.completed {
                    task.completed
                } else if filters.pending {
                    !task.completed
                } else {
                    true
                };

                category_match && priority_match && completion_match
            })
            .collect()
    }

    fn complete_task(&mut self, id: usize) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.completed = true;
            self.save()?;
            Ok(())
        } else {
            Err(format!("Task with ID {} not found", id))
        }
    }

    fn remove_task(&mut self, id: usize) -> Result<(), String> {
        if let Some(index) = self.tasks.iter().position(|t| t.id == id) {
            self.tasks.remove(index);
            self.save()?;
            Ok(())
        } else {
            Err(format!("Task with ID {} not found", id))
        }
    }

    fn edit_task(&mut self, id: usize, updates: TaskUpdates) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if let Some(title) = updates.title {
                task.title = title;
            }
            if let Some(description) = updates.description {
                task.description = Some(description);
            }
            if let Some(due_date) = updates.due {
                task.due_date = Some(
                    NaiveDateTime::parse_from_str(&due_date, "%Y-%m-%d %H:%M")
                        .map_err(|e| format!("Invalid date format: {}", e))?,
                );
            }
            if let Some(priority) = updates.priority {
                task.priority = match priority.to_lowercase().as_str() {
                    "low" => Priority::Low,
                    "medium" => Priority::Medium,
                    "high" => Priority::High,
                    "critical" => Priority::Critical,
                    _ => return Err("Invalid priority level".to_string()),
                };
            }
            if let Some(category) = updates.category {
                task.category = match category.to_lowercase().as_str() {
                    "personal" => Category::Personal,
                    "work" => Category::Work,
                    "shopping" => Category::Shopping,
                    "health" => Category::Health,
                    other => Category::Other(other.to_string()),
                };
            }
            if let Some(tags) = updates.tags {
                task.tags = tags.split(',').map(|s| s.trim().to_string()).collect();
            }
            self.save()?;
            Ok(())
        } else {
            Err(format!("Task with ID {} not found", id))
        }
    }

    fn show_task(&self, id: usize) -> Result<&Task, String> {
        self.tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Task with ID {} not found", id))
    }
}

struct ListFilters {
    category: Option<String>,
    priority: Option<String>,
    completed: bool,
    pending: bool,
}

struct TaskUpdates {
    title: Option<String>,
    description: Option<String>,
    due: Option<String>,
    priority: Option<String>,
    category: Option<String>,
    tags: Option<String>,
}

fn display_tasks(tasks: Vec<&Task>) {
    if tasks.is_empty() {
        println!("No tasks found.");
        return;
    }

    let mut table = Table::new();
    table.add_row(row![
        "ID",
        "Status",
        "Title",
        "Due Date",
        "Priority",
        "Category",
        "Tags"
    ]);

    for task in tasks {
        table.add_row(task.to_row());
    }

    table.printstd();
}

fn display_task_details(task: &Task) {
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new(&task.id.to_string()),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Title"),
        Cell::new(&task.title),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Description"),
        Cell::new(task.description.as_deref().unwrap_or("-")),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Status"),
        Cell::new(if task.completed {
            "Completed".green()
        } else {
            "Pending".red()
        }),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Created"),
        Cell::new(&task.created_at.format("%Y-%m-%d %H:%M").to_string()),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Due Date"),
        Cell::new(
            &task
                .due_date
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Priority"),
        Cell::new(&format!("{:?}", task.priority)),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Category"),
        Cell::new(&format!("{:?}", task.category)),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Tags"),
        Cell::new(&task.tags.join(", ")),
    ]));

    table.printstd();
}

fn main() {
    let cli = Cli::parse();
    let mut manager = TodoManager::new();

    match cli.command {
        Commands::Add {
            title,
            description,
            due,
            priority,
            category,
            tags,
        } => {
            let id = manager.tasks.len() + 1;
            match Task::new(id, title, description, due, &priority, &category, tags) {
                Ok(task) => {
                    if let Err(e) = manager.add_task(task) {
                        eprintln!("Error adding task: {}", e);
                    } else {
                        println!("Task added successfully!");
                    }
                }
                Err(e) => eprintln!("Error creating task: {}", e),
            }
        }
        Commands::List {
            category,
            priority,
            completed,
            pending,
        } => {
            let filters = ListFilters {
                category,
                priority,
                completed,
                pending,
            };
            display_tasks(manager.list_tasks(&filters));
        }
        Commands::Show { id } => match manager.show_task(id) {
            Ok(task) => display_task_details(task),
            Err(e) => eprintln!("Error showing task: {}", e),
        },
        Commands::Complete { id } => {
            if let Err(e) = manager.complete_task(id) {
                eprintln!("Error completing task: {}", e);
            } else {
                println!("Task completed successfully!");
            }
        }
        Commands::Remove { id } => {
            if let Err(e) = manager.remove_task(id) {
                eprintln!("Error removing task: {}", e);
            } else {
                println!("Task removed successfully!");
            }
        }
        Commands::Edit {
            id,
            title,
            description,
            due,
            priority,
            category,
            tags,
        } => {
            let updates = TaskUpdates {
                title,
                description,
                due,
                priority,
                category,
                tags,
            };
            if let Err(e) = manager.edit_task(id, updates) {
                eprintln!("Error editing task: {}", e);
            } else {
                println!("Task updated successfully!");
            }
        }
    }
}
                
                