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
            password_hash: "hashed_password".to_string(),
            full_name: Some("Test User".to_string()),
            avatar_s3key: None,
            is_active: Some(true),
            is_admin: Some(false),
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
                Some("new_hash"),
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
                password_hash: "hash".to_string(),
                full_name: Some(format!("User {}", i)),
                avatar_s3key: None,
                is_active: Some(true),
                is_admin: Some(false),
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
            password_hash: "hash".to_string(),
            full_name: None,
            avatar_s3key: None,
            is_active: Some(true),
            is_admin: Some(false),
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
}
