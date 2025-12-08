// Simple integration tests for SQL operations
// These tests verify the basic CRUD operations work correctly

#[cfg(test)]
mod integration_tests {
    use crate::db::sql::{
        models::{NewUser, NewAuthor, NewCitation, NewPublication}, 
        SqlClient, 
        UserOperations,
        AuthorOperations,
        CitationOperations,
        PublicationOperations,
        PublicationAuthorOperations
    };
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_user_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Test create user
        let new_user = NewUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: Some("Test User".to_string()),
            avatar_s3key: None,
            is_active: Some(true),
            is_admin: Some(false),
            privy_id: "privy_test_user_123".to_string(),
        };

        let user = sql_client.create_user(&new_user).await?;
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");

        // Test get user
        let retrieved_user = sql_client.get_user(user.id).await?;
        assert_eq!(retrieved_user.id, user.id);
        assert_eq!(retrieved_user.username, "testuser");

        // Test update user
        let result = sql_client
            .update_user(
                user.id,
                Some("updateduser"),
                Some("updated@example.com"),
                Some("Updated User"),
                None,
                Some(false),
                Some(true),
            )
            .await?;
        assert!(result.rows_affected() > 0);

        // Test delete user
        let delete_result = sql_client.delete_user(user.id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_user_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create multiple users
        for i in 0..3 {
            let new_user = NewUser {
                username: format!("user{}", i),
                email: format!("user{}@example.com", i),
                full_name: Some(format!("User {}", i)),
                avatar_s3key: None,
                is_active: Some(true),
                is_admin: Some(false),
                privy_id: format!("privy_user_{}", i),
            };
            sql_client.create_user(&new_user).await?;
        }

        // Test listing users
        let users = sql_client.list_users(Some(1), Some(10)).await?;
        assert_eq!(users.len(), 3);

        // Test counting users
        let count = sql_client.count_users().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_user_email_exists(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let new_user = NewUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: None,
            avatar_s3key: None,
            is_active: Some(true),
            is_admin: Some(false),
            privy_id: "privy_test_user_456".to_string(),
        };

        sql_client.create_user(&new_user).await?;

        // Test email exists
        let exists = sql_client.user_email_exists("test@example.com").await?;
        assert!(exists);

        // Test non-existent email
        let not_exists = sql_client.user_email_exists("nonexistent@example.com").await?;
        assert!(!not_exists);

        Ok(())
    }

    #[sqlx::test]
    async fn test_author_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Test create author
        let new_author = NewAuthor {
            name: "Test Author".to_string(),
            email: Some("author@example.com".to_string()),
            affiliation: Some("Test University".to_string()),
        };

        let author = sql_client.create_author(&new_author).await?;
        assert_eq!(author.name, "Test Author");
        assert_eq!(author.email, Some("author@example.com".to_string()));

        // Test get author
        let retrieved_author = sql_client.get_author(author.id).await?;
        assert_eq!(retrieved_author.id, author.id);
        assert_eq!(retrieved_author.name, "Test Author");

        // Test update author
        let result = sql_client
            .update_author(
                author.id,
                Some("Updated Author"),
                Some("updated@example.com"),
                Some("Updated University"),
            )
            .await?;
        assert!(result.rows_affected() > 0);

        // Test delete author
        let delete_result = sql_client.delete_author(author.id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_author_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create multiple authors
        for i in 0..3 {
            let new_author = NewAuthor {
                name: format!("Author {}", i),
                email: Some(format!("author{}@example.com", i)),
                affiliation: Some(format!("University {}", i)),
            };
            sql_client.create_author(&new_author).await?;
        }

        // Test listing authors
        let authors = sql_client.list_authors(Some(1), Some(10)).await?;
        assert_eq!(authors.len(), 3);

        // Test counting authors
        let count = sql_client.count_authors().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // First create two publications to cite (without file handling)
        let pub1 = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Citing Publication".to_string(),
                about: Some("This publication cites another".to_string()),
                tags: Some(vec!["citing".to_string()]),
                s3key: None,
            })
            .await?;

        let pub2 = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Cited Publication".to_string(),
                about: Some("This publication is cited".to_string()),
                tags: Some(vec!["cited".to_string()]),
                s3key: None,
            })
            .await?;

        // Test create citation
        let new_citation = NewCitation {
            citing_publication_id: pub1.id,
            cited_publication_id: pub2.id,
            citation_context: Some("This is an important reference".to_string()),
        };

        let citation = sql_client.create_citation(&new_citation).await?;
        assert_eq!(citation.citing_publication_id, pub1.id);
        assert_eq!(citation.cited_publication_id, pub2.id);

        // Test get citation
        let retrieved_citation = sql_client.get_citation(citation.id).await?;
        assert_eq!(retrieved_citation.id, citation.id);

        // Test get citation by publications
        let citation_by_pubs = sql_client
            .get_citation_by_publications(pub1.id, pub2.id)
            .await?;
        assert!(citation_by_pubs.is_some());
        assert_eq!(citation_by_pubs.unwrap().id, citation.id);

        // Test update citation
        let result = sql_client
            .update_citation(citation.id, Some("Updated context"))
            .await?;
        assert!(result.rows_affected() > 0);

        // Test delete citation
        let delete_result = sql_client.delete_citation(citation.id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create publications sequentially
        let mut pub_results = Vec::new();
        for i in 0..4 {
            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await?;
            pub_results.push(publication);
        }

        // Create multiple citations
        for i in 0..3 {
            let new_citation = NewCitation {
                citing_publication_id: pub_results[i].id,
                cited_publication_id: pub_results[i + 1].id,
                citation_context: Some(format!("Citation {}", i)),
            };
            sql_client.create_citation(&new_citation).await?;
        }

        // Test listing citations
        let citations = sql_client.list_citations(Some(1), Some(10)).await?;
        assert_eq!(citations.len(), 3);

        // Test counting citations
        let count = sql_client.count_citations().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_author_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create a publication and an author
        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Test Publication".to_string(),
                about: None,
                tags: None,
                s3key: None,
            })
            .await?;

        let author = sql_client
            .create_author(&NewAuthor {
                name: "Test Author".to_string(),
                email: Some("author@example.com".to_string()),
                affiliation: Some("Test University".to_string()),
            })
            .await?;

        // Test add author to publication
        sql_client.add_author_to_publication(publication.id, author.id, Some(1)).await?;

        // Test get publication authors (using PublicationAuthorOperations trait)
        let pub_authors = PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id).await?;
        assert_eq!(pub_authors.len(), 1);
        assert_eq!(pub_authors[0].author_id, author.id);

        // Test publication has author
        let has_author = sql_client.publication_has_author(publication.id, author.id).await?;
        assert!(has_author);

        // Test count authors for publication
        let author_count = sql_client.count_authors_for_publication(publication.id).await?;
        assert_eq!(author_count, 1);

        // Test remove author from publication
        let remove_result = sql_client.remove_author_from_publication(publication.id, author.id).await?;
        assert!(remove_result.rows_affected() > 0);

        // Verify author removed
        let pub_authors_after = PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id).await?;
        assert!(pub_authors_after.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_set_publication_authors(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create a publication and multiple authors
        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Test Publication".to_string(),
                about: None,
                tags: None,
                s3key: None,
            })
            .await?;

        // Create authors sequentially
        let mut author_results = Vec::new();
        for i in 0..3 {
            let author = sql_client
                .create_author(&NewAuthor {
                    name: format!("Author {}", i),
                    email: Some(format!("author{}@example.com", i)),
                    affiliation: Some(format!("University {}", i)),
                })
                .await?;
            author_results.push(author);
        }

        // Test set publication authors
        let author_ids: Vec<Uuid> = author_results.iter().map(|a| a.id).collect();
        sql_client.set_publication_authors(publication.id, &author_ids).await?;

        // Verify authors were set (using PublicationAuthorOperations trait)
        let pub_authors = PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id).await?;
        assert_eq!(pub_authors.len(), 3);

        // Verify ordering
        for (i, pub_author) in pub_authors.iter().enumerate() {
            assert_eq!(pub_author.author_id, author_ids[i]);
            assert_eq!(pub_author.author_order, (i + 1) as i32);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_search_publications_by_title(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create publications with different titles
        let publications = vec![
            "Machine Learning Advances",
            "Deep Learning Research",
            "Artificial Intelligence Review",
            "Machine Vision Systems",
        ];

        for title in publications {
            sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: title.to_string(),
                    about: Some("Test description".to_string()),
                    tags: Some(vec!["ai".to_string()]),
                    s3key: None,
                })
                .await?;
        }

        // Search for publications with "Machine" in title
        let machine_pubs = sql_client
            .search_publications_by_title("Machine", Some(1), Some(10))
            .await?;

        // Should return 2 publications with "Machine" in title
        assert_eq!(machine_pubs.len(), 2);
        for pub_item in &machine_pubs {
            assert!(pub_item.title.contains("Machine"));
        }

        // Search for publications with "Learning" in title
        let learning_pubs = sql_client
            .search_publications_by_title("Learning", Some(1), Some(10))
            .await?;

        // Should return 2 publications with "Learning" in title
        assert_eq!(learning_pubs.len(), 2);
        for pub_item in &learning_pubs {
            assert!(pub_item.title.contains("Learning"));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_search_publications_by_tag(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create publications with different tags
        let publications = vec![
            ("Paper 1", vec!["ai".to_string(), "ml".to_string()]),
            ("Paper 2", vec!["ml".to_string(), "dl".to_string()]),
            ("Paper 3", vec!["ai".to_string(), "cv".to_string()]),
            ("Paper 4", vec!["nlp".to_string()]),
        ];

        for (title, tags) in publications {
            sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: title.to_string(),
                    about: Some("Test description".to_string()),
                    tags: Some(tags),
                    s3key: None,
                })
                .await?;
        }

        // Search for publications with "ai" tag
        let ai_pubs = sql_client
            .search_publications_by_tag("ai", Some(1), Some(10))
            .await?;

        // Should return 2 publications with "ai" tag
        assert_eq!(ai_pubs.len(), 2);
        for pub_item in &ai_pubs {
            assert!(pub_item.tags.contains(&"ai".to_string()));
        }

        // Search for publications with "ml" tag
        let ml_pubs = sql_client
            .search_publications_by_tag("ml", Some(1), Some(10))
            .await?;

        // Should return 2 publications with "ml" tag
        assert_eq!(ml_pubs.len(), 2);
        for pub_item in &ml_pubs {
            assert!(pub_item.tags.contains(&"ml".to_string()));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_relationships(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create multiple publications
        let mut publications = Vec::new();
        for i in 0..5 {
            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await?;
            publications.push(publication);
        }

        // Create citation relationships: 0 cites 1, 1 cites 2, 2 cites 3, 3 cites 4
        for i in 0..4 {
            let new_citation = NewCitation {
                citing_publication_id: publications[i].id,
                cited_publication_id: publications[i + 1].id,
                citation_context: Some(format!("Citation from {} to {}", i, i + 1)),
            };
            sql_client.create_citation(&new_citation).await?;
        }

        // Test get_publication_citations for publication 1
        let pub1_citations = sql_client.get_publication_citations(publications[1].id).await?;
        // Should return 2 citations (cited by 0, cites 2)
        assert_eq!(pub1_citations.len(), 2);

        // Test get_cited_by for publication 2
        let cited_by = sql_client.get_cited_by(publications[2].id).await?;
        // Should return 1 publication (cited by 1)
        assert_eq!(cited_by.len(), 1);
        assert_eq!(cited_by[0].id, publications[1].id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_author_publications_relationship(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create an author
        let author = sql_client
            .create_author(&NewAuthor {
                name: "Test Author".to_string(),
                email: Some("author@example.com".to_string()),
                affiliation: Some("Test University".to_string()),
            })
            .await?;

        // Create multiple publications
        let mut publications = Vec::new();
        for i in 0..5 {
            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await?;
            publications.push(publication);
        }

        // Add author to first 3 publications
        for i in 0..3 {
            sql_client
                .add_author_to_publication(publications[i].id, author.id, Some(i as i32))
                .await?;
        }

        // Test get_author_publications
        let author_pubs = sql_client
            .get_author_publications(author.id, Some(1), Some(10))
            .await?;

        // Should return 3 publications
        assert_eq!(author_pubs.len(), 3);

        // Test count_publications_for_author
        let pub_count = sql_client.count_publications_for_author(author.id).await?;
        assert_eq!(pub_count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_author_order(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create a publication and an author
        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Test Publication".to_string(),
                about: None,
                tags: None,
                s3key: None,
            })
            .await?;

        let author = sql_client
            .create_author(&NewAuthor {
                name: "Test Author".to_string(),
                email: Some("author@example.com".to_string()),
                affiliation: Some("Test University".to_string()),
            })
            .await?;

        // Add author with initial order 1
        sql_client.add_author_to_publication(publication.id, author.id, Some(1)).await?;

        // Update author order to 2
        let result = sql_client
            .update_author_order(publication.id, author.id, 2)
            .await?;
        assert!(result.rows_affected() > 0);

        // Verify the update
        let pub_authors = PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id).await?;
        assert_eq!(pub_authors.len(), 1);
        assert_eq!(pub_authors[0].author_order, 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Test create publication
        let new_publication = NewPublication {
            user_id: None,
            title: "Test Publication".to_string(),
            about: Some("This is a test publication".to_string()),
            tags: Some(vec!["test".to_string(), "ai".to_string()]),
            s3key: Some("s3://bucket/key.pdf".to_string()),
        };

        let publication = sql_client.create_publication(&new_publication).await?;
        assert_eq!(publication.title, "Test Publication");
        assert_eq!(publication.about, Some("This is a test publication".to_string()));
        assert_eq!(publication.tags, vec!["test".to_string(), "ai".to_string()]);
        assert_eq!(publication.s3key, Some("s3://bucket/key.pdf".to_string()));

        // Test get publication
        let retrieved_publication = sql_client.get_publication(publication.id).await?;
        assert_eq!(retrieved_publication.id, publication.id);
        assert_eq!(retrieved_publication.title, "Test Publication");

        // Test update publication
        let update_result = sql_client
            .update_publication(
                publication.id,
                None, // user_id stays the same
                Some("Updated Title"),
                Some("Updated description"),
                Some(&["updated".to_string(), "ml".to_string()]),
                Some("s3://bucket/updated.pdf"),
            )
            .await?;
        assert!(update_result.rows_affected() > 0);

        // Verify update
        let updated_publication = sql_client.get_publication(publication.id).await?;
        assert_eq!(updated_publication.title, "Updated Title");
        assert_eq!(updated_publication.about, Some("Updated description".to_string()));
        assert_eq!(updated_publication.tags, vec!["updated".to_string(), "ml".to_string()]);
        assert_eq!(updated_publication.s3key, Some("s3://bucket/updated.pdf".to_string()));

        // Test delete publication
        let delete_result = sql_client.delete_publication(publication.id).await?;
        assert!(delete_result.rows_affected() > 0);

        // Verify deletion (should error)
        let deleted_result = sql_client.get_publication(publication.id).await;
        assert!(deleted_result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_publications_by_user(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create a user
        let user = sql_client
            .create_user(&NewUser {
                username: "testuser".to_string(),
                email: "user@example.com".to_string(),
                full_name: Some("Test User".to_string()),
                avatar_s3key: None,
                is_active: Some(true),
                is_admin: Some(false),
                privy_id: "privy_test_user_789".to_string(),
            })
            .await?;

        // Create publications for the user
        for i in 0..3 {
            sql_client
                .create_publication(&NewPublication {
                    user_id: Some(user.id),
                    title: format!("User Publication {}", i),
                    about: Some("User publication description".to_string()),
                    tags: Some(vec!["user".to_string()]),
                    s3key: None,
                })
                .await?;
        }

        // Create publications without user (should not appear in user list)
        for i in 0..2 {
            sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Anonymous Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await?;
        }

        // Test list publications by user
        let user_publications = sql_client
            .list_publications_by_user(user.id, Some(1), Some(10))
            .await?;

        // Should return only the 3 publications for this user
        assert_eq!(user_publications.len(), 3);
        for pub_item in &user_publications {
            assert_eq!(pub_item.user_id, Some(user.id));
            assert!(pub_item.title.starts_with("User Publication"));
        }

        // Test count publications by user
        let user_pub_count = sql_client.count_publications_by_user(user.id).await?;
        assert_eq!(user_pub_count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_and_count_publications(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create multiple publications
        for i in 0..5 {
            sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Publication {}", i),
                    about: Some(format!("Description {}", i)),
                    tags: Some(vec!["test".to_string()]),
                    s3key: None,
                })
                .await?;
        }

        // Test list publications with pagination
        let page1 = sql_client.list_publications(Some(1), Some(3)).await?;
        assert_eq!(page1.len(), 3);

        let page2 = sql_client.list_publications(Some(2), Some(3)).await?;
        assert_eq!(page2.len(), 2);

        // Test count publications
        let total_count = sql_client.count_publications().await?;
        assert_eq!(total_count, 5);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_pagination_edge_cases(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create many publications
        for i in 0..25 {
            sql_client
                .create_publication(&NewPublication {
                    user_id: None,
                    title: format!("Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await?;
        }

        // Test default pagination (page 1, limit 20)
        let default_page = sql_client.list_publications(None, None).await?;
        assert_eq!(default_page.len(), 20);

        // Test page beyond available data
        let empty_page = sql_client.list_publications(Some(10), Some(10)).await?;
        assert!(empty_page.is_empty());

        // Test small limit
        let small_page = sql_client.list_publications(Some(1), Some(5)).await?;
        assert_eq!(small_page.len(), 5);

        // Test large limit
        let large_page = sql_client.list_publications(Some(1), Some(100)).await?;
        assert_eq!(large_page.len(), 25);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_update_partial_fields(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        // Create a publication
        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: None,
                title: "Original Title".to_string(),
                about: Some("Original description".to_string()),
                tags: Some(vec!["original".to_string()]),
                s3key: Some("s3://original.pdf".to_string()),
            })
            .await?;

        // Test updating only title
        let result1 = sql_client
            .update_publication(
                publication.id,
                None,
                Some("Updated Title Only"),
                None,
                None,
                None,
            )
            .await?;
        assert!(result1.rows_affected() > 0);

        let after_title_update = sql_client.get_publication(publication.id).await?;
        assert_eq!(after_title_update.title, "Updated Title Only");
        assert_eq!(after_title_update.about, Some("Original description".to_string()));
        assert_eq!(after_title_update.tags, vec!["original".to_string()]);
        assert_eq!(after_title_update.s3key, Some("s3://original.pdf".to_string()));

        // Test updating only tags
        let result2 = sql_client
            .update_publication(
                publication.id,
                None,
                None,
                None,
                Some(&["updated".to_string(), "tags".to_string()]),
                None,
            )
            .await?;
        assert!(result2.rows_affected() > 0);

        let after_tags_update = sql_client.get_publication(publication.id).await?;
        assert_eq!(after_tags_update.title, "Updated Title Only");
        assert_eq!(after_tags_update.tags, vec!["updated".to_string(), "tags".to_string()]);

        // Test updating only s3key
        let result3 = sql_client
            .update_publication(
                publication.id,
                None,
                None,
                None,
                None,
                Some("s3://updated.pdf"),
            )
            .await?;
        assert!(result3.rows_affected() > 0);

        let after_s3key_update = sql_client.get_publication(publication.id).await?;
        assert_eq!(after_s3key_update.s3key, Some("s3://updated.pdf".to_string()));

        Ok(())
    }
}
