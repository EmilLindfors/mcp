use std::{path::{PathBuf, Path}, collections::HashMap};
use tokio::fs;
use serde::{Deserialize, Serialize};
use crate::{
    error::McpError,
    protocol::{Protocol, ProtocolOptions},
    transport::StdioTransport,
    types::{Tool, ToolInputSchema, SchemaProperty, ListToolsResponse, CallToolResponse, ToolContent},
};

pub struct FileSystemServer {
    allowed_directories: Vec<PathBuf>,
    protocol: Protocol,
}

#[derive(Debug, Deserialize)]
struct ReadFileArgs {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ReadMultipleFilesArgs {
    paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CreateDirectoryArgs {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ListDirectoryArgs {
    path: String,
}

#[derive(Debug, Deserialize)]
struct MoveFileArgs {
    source: String,
    destination: String,
}

#[derive(Debug, Deserialize)]
struct SearchFilesArgs {
    path: String,
    pattern: String,
}

#[derive(Debug, Deserialize)]
struct GetFileInfoArgs {
    path: String,
}

#[derive(Debug, Serialize)]
struct FileInfo {
    size: u64,
    created: String,
    modified: String,
    accessed: String,
    is_directory: bool,
    is_file: bool,
    permissions: String,
}

impl FileSystemServer {
    pub fn new(allowed_dirs: Vec<PathBuf>) -> Self {
        let protocol = Protocol::builder(Some(ProtocolOptions {
            enforce_strict_capabilities: true,
        }))
        .build();

        Self {
            allowed_directories: allowed_dirs,
            protocol,
        }
    }

    async fn validate_path(&self, requested_path: &str) -> Result<PathBuf, McpError> {
        let requested_path = PathBuf::from(requested_path);
        let absolute = if requested_path.is_absolute() {
            requested_path.clone()
        } else {
            std::env::current_dir().unwrap().join(requested_path)
        };

        let normalized = absolute.canonicalize().map_err(|_| McpError::IoError)?;
        
        for allowed_dir in &self.allowed_directories {
            if normalized.starts_with(allowed_dir) {
                return Ok(normalized);
            }
        }

        Err(McpError::IoError)
    }

    async fn get_file_stats(&self, path: &Path) -> Result<FileInfo, McpError> {
        let metadata = fs::metadata(path).await.unwrap();
        
        Ok(FileInfo {
            size: metadata.len(),
            created: metadata.created().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string(),
            modified: metadata.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string(),
            accessed: metadata.accessed().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string(),
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            permissions: format!("{:?}", metadata.permissions()),
        })
    }

    async fn search_files(&self, root: &Path, pattern: &str) -> Result<Vec<PathBuf>, McpError> {
        let mut results = Vec::new();
        let pattern = pattern.to_lowercase();

        Self::search_dir(root, &pattern, &mut results).await.unwrap();
        Ok(results)
    }

    async fn search_dir(dir: &Path, pattern: &str, results: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase().contains(pattern))
                .unwrap_or(false)
            {
                results.push(path.clone());
            }
            if path.is_dir() {
                panic!("Not implemented");
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), McpError> {
        let transport = StdioTransport::new(32);
        let tools = self.create_tools();

        // Register handlers
        self.register_handlers().await?;

        // Connect transport
        self.protocol.connect(transport).await?;

        // Keep running
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    fn create_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "read_file".to_string(),
                description: "Read the complete contents of a file from the file system. \
                    Handles various text encodings and provides detailed error messages \
                    if the file cannot be read. Use this tool when you need to examine \
                    the contents of a single file. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string()],
                },
            },
            Tool {
                name: "read_multiple_files".to_string(),
                description: "Read the contents of multiple files simultaneously. This is more \
                    efficient than reading files one by one when you need to analyze \
                    or compare multiple files. Each file's content is returned with its \
                    path as a reference. Failed reads for individual files won't stop \
                    the entire operation. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("paths".to_string(), SchemaProperty {
                            property_type: "array".to_string(),
                            items: Some(Box::new(SchemaProperty {
                                property_type: "string".to_string(),
                                items: None,
                            })),
                        });
                        map
                    },
                    required: vec!["paths".to_string()],
                },
            },
            Tool {
                name: "write_file".to_string(),
                description: "Create a new file or completely overwrite an existing file with new content. \
                    Use with caution as it will overwrite existing files without warning. \
                    Handles text content with proper encoding. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map.insert("content".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string(), "content".to_string()],
                },
            },
            Tool {
                name: "create_directory".to_string(),
                description: "Create a new directory or ensure a directory exists. Can create multiple \
                    nested directories in one operation. If the directory already exists, \
                    this operation will succeed silently. Perfect for setting up directory \
                    structures for projects or ensuring required paths exist. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string()],
                },
            },
            Tool {
                name: "list_directory".to_string(),
                description: "Get a detailed listing of all files and directories in a specified path. \
                    Results clearly distinguish between files and directories with [FILE] and [DIR] \
                    prefixes. This tool is essential for understanding directory structure and \
                    finding specific files within a directory. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string()],
                },
            },
            Tool {
                name: "move_file".to_string(),
                description: "Move or rename files and directories. Can move files between directories \
                    and rename them in a single operation. If the destination exists, the \
                    operation will fail. Works across different directories and can be used \
                    for simple renaming within the same directory. Both source and destination must be within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("source".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map.insert("destination".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["source".to_string(), "destination".to_string()],
                },
            },
            Tool {
                name: "search_files".to_string(),
                description: "Recursively search for files and directories matching a pattern. \
                    Searches through all subdirectories from the starting path. The search \
                    is case-insensitive and matches partial names. Returns full paths to all \
                    matching items. Great for finding files when you don't know their exact location. \
                    Only searches within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map.insert("pattern".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string(), "pattern".to_string()],
                },
            },
            Tool {
                name: "get_file_info".to_string(),
                description: "Retrieve detailed metadata about a file or directory. Returns comprehensive \
                    information including size, creation time, last modified time, permissions, \
                    and type. This tool is perfect for understanding file characteristics \
                    without reading the actual content. Only works within allowed directories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut map = HashMap::new();
                        map.insert("path".to_string(), SchemaProperty {
                            property_type: "string".to_string(),
                            items: None,
                        });
                        map
                    },
                    required: vec!["path".to_string()],
                },
            },
            Tool {
                name: "list_allowed_directories".to_string(),
                description: "Returns the list of directories that this server is allowed to access. \
                    Use this to understand which directories are available before trying to access files.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        ]
    }

    async fn register_handlers(&mut self) -> Result<(), McpError> {
        let tools = self.create_tools();
        
        // Register list_tools handler
        self.protocol.set_request_handler(
            "tools/list",
            Box::new(move |_req, _extra| {
                let tools = tools.clone();
                Box::pin(async move {
                    Ok(serde_json::to_value(ListToolsResponse { tools }).unwrap())
                })
            }),
        ).await;

        // Register call_tool handler
        let allowed_dirs = self.allowed_directories.clone();
        self.protocol.set_request_handler(
            "tools/call",
            Box::new(move |req, _extra| {
                let allowed_dirs = allowed_dirs.clone();
                Box::pin(async move {
                    // Tool call implementation here
                    Ok(serde_json::json!({}))
                })
            }),
        ).await;

        Ok(())
    }
}
