use serde::{Deserialize, Serialize};

use crate::{error::Error, image_info::ImageInfo, IMAGE_OFFSET};

#[derive(Debug, Clone)]
pub enum TreeNode {
    Branch { a: Box<TreeNode>, b: Box<TreeNode> },
    Leaf { images: Vec<ImageInfo> },
}

impl TreeNode {
    pub fn tree_from_images(images: &Vec<ImageInfo>) -> Option<TreeNode> {
        if images.is_empty() {
            return None;
        }

        let levels = images.len().ilog2() + 1; // +1 for the remainder
        println!("Levels: {}", levels);

        let root = TreeNode::split(&TreeNode::Leaf {
            images: images.clone(),
        });

        Some(root)
    }

    pub fn split(node: &TreeNode) -> TreeNode {
        match node {
            TreeNode::Branch { a, b } => TreeNode::Branch {
                a: Box::new(TreeNode::split(a)),
                b: Box::new(TreeNode::split(b)),
            },
            TreeNode::Leaf { images } => {
                if images.len() <= 2 {
                    return TreeNode::Leaf {
                        images: images.clone(),
                    };
                }
                let parts = images.split_at(images.len() / 2);
                TreeNode::Branch {
                    a: Box::new(TreeNode::split(&TreeNode::Leaf {
                        images: parts.0.to_vec(),
                    })),
                    b: Box::new(TreeNode::split(&TreeNode::Leaf {
                        images: parts.1.to_vec(),
                    })),
                }
            }
        }
    }

    pub fn height(&self) -> u64 {
        match self {
            TreeNode::Branch { a, b } => a.height() + b.height(),
            TreeNode::Leaf { images } => images.iter().map(|i| i.height + IMAGE_OFFSET).sum(),
        }
    }

    pub fn path(&self, path: &str) -> Result<&Self, Error> {
        if path.is_empty() {
            return Ok(self);
        }

        match self {
            TreeNode::Branch { a, b } => {
                let mut char_iter = path.chars();
                let next = char_iter.next().unwrap();
                let rest = char_iter.collect::<String>();
                if next == 'a' {
                    a.path(&rest)
                } else if next == 'b' {
                    b.path(&rest)
                } else {
                    Err(Error::InvalidPath(path.to_string()))
                }
            }
            // we've got to a leaf node but the path isn't empty? return it.
            TreeNode::Leaf { images: _ } => Ok(self),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TreeNodeLayer {
    Branch {
        a_height: u64,
        b_height: u64,
    },
    Images {
        a_height: u64,
        a_width: u64,
        b_height: u64,
        b_width: u64,
        a_url: String,
        b_url: String,
    },
}

impl From<&TreeNode> for TreeNodeLayer {
    fn from(value: &TreeNode) -> Self {
        match value {
            TreeNode::Branch { a, b } => TreeNodeLayer::Branch {
                a_height: a.height(),
                b_height: b.height(),
            },
            TreeNode::Leaf { images } => TreeNodeLayer::Images {
                a_height: images[0].height,
                a_width: images[0].width,
                b_height: images[1].height,
                b_width: images[1].width,
                a_url: images[0].url.clone(),
                b_url: images[1].url.clone(),
            },
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::time::SystemTime;

    use crate::{image_info::ImageInfo, tree::TreeNode, IMAGE_OFFSET};

    fn simple_image(height: u64) -> ImageInfo {
        ImageInfo {
            url: "0000-0000-33333-33333.png".to_string(),
            source: "fish.png".to_string(),
            date: SystemTime::now(),
            is_video: false,
            width: 600,
            height: height,
            height_before: 0,
            height_after: 0,
        }
    }

    #[test]
    fn test_build_tree_single_image() {
        let image_list = vec![simple_image(400)];

        let result = TreeNode::tree_from_images(&image_list);

        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.height(), 400 + IMAGE_OFFSET);
    }

    #[test]
    fn test_build_tree_single_four() {
        let image_list = vec![
            simple_image(400),
            simple_image(500),
            simple_image(300),
            simple_image(600),
        ];

        let result = TreeNode::tree_from_images(&image_list);

        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.height(), 1800 + IMAGE_OFFSET * 4);

        match node.path("a").unwrap() {
            TreeNode::Leaf { images } => {
                assert_eq!(images.len(), 2);
                assert_eq!(images[0].height, 400);
                assert_eq!(images[1].height, 500);
            }
            _ => assert!(false, "invalid node"),
        }
        match node.path("b").unwrap() {
            TreeNode::Leaf { images } => {
                assert_eq!(images.len(), 2);
                assert_eq!(images[0].height, 300);
                assert_eq!(images[1].height, 600);
            }
            _ => assert!(false, "invalid node"),
        }
    }

    #[test]
    fn test_tree_node_ten() {
        let mut image_list = Vec::new();
        for i in (100..=1000).step_by(100) {
            image_list.push(simple_image(i))
        }

        let result = TreeNode::tree_from_images(&image_list);

        assert!(result.is_some());
        let node = result.unwrap();
        assert_eq!(node.height(), 5500 + (IMAGE_OFFSET * 10));

        match node.path("bbb").unwrap() {
            TreeNode::Leaf { images } => {
                assert_eq!(images.len(), 2);
                assert_eq!(images[0].height, 900);
                assert_eq!(images[1].height, 1000);
            }
            _ => assert!(false, "invalid node type"),
        }
    }
}
