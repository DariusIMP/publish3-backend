// Simple integration tests for SQL operations
// These tests verify the basic CRUD operations work correctly

#[cfg(test)]
mod integration_tests {
    use crate::db::sql::{
        AuthorOperations, CitationOperations, PublicationAuthorOperations, PublicationOperations,
        SqlClient, UserOperations,
        models::{NewAuthor, NewCitation, NewPublication, NewUser},
    };
    use uuid::Uuid;

    async fn create_test_user(sql_client: &SqlClient, prefix: &str) -> sqlx::Result<String> {
        let privy_id = format!("{}_{}", prefix, Uuid::new_v4());
        let new_user = NewUser {
            privy_id: privy_id.clone(),
        };
        sql_client.create_user(&new_user).await?;
        Ok(privy_id)
    }

    async fn create_test_publication(
        sql_client: &SqlClient,
        user_privy_id: &str,
        title: Option<&str>,
    ) -> sqlx::Result<crate::db::sql::models::Publication> {
        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: user_privy_id.to_string(),
                title: title.unwrap_or("Test Publication").to_string(),
                about: "Test description".to_string(),
                tags: vec!["test".to_string()],
                s3key: "".to_string(),
                price: 0,
                citation_royalty_bps: 0,
            })
            .await?;
        Ok(publication)
    }

    async fn create_test_author(
        sql_client: &SqlClient,
        user_privy_id: &str,
    ) -> sqlx::Result<crate::db::sql::models::Author> {
        let author = sql_client
            .create_author(&NewAuthor {
                privy_id: user_privy_id.to_string(),
                name: format!("Test Author {}", user_privy_id),
                email: Some(format!("author_{}@example.com", user_privy_id)),
                affiliation: Some("Test University".to_string()),
                wallet_address: format!("0x{}", Uuid::new_v4()),
            })
            .await?;
        Ok(author)
    }

    #[sqlx::test]
    async fn test_user_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let privy_id = create_test_user(&sql_client, "test").await?;

        let user = sql_client.get_user(privy_id.clone()).await?;
        assert_eq!(user.privy_id, privy_id);

        let delete_result = sql_client.delete_user(privy_id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_user_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        for i in 0..3 {
            let new_user = NewUser {
                privy_id: format!("privy_user_{}", i),
            };
            sql_client.create_user(&new_user).await?;
        }

        let users = sql_client.list_users(Some(1), Some(10)).await?;
        assert_eq!(users.len(), 3);

        let count = sql_client.count_users().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_user_email_exists(pool: sqlx::PgPool) -> sqlx::Result<()> {
        // This test is no longer relevant since we don't store email in users table
        // We'll skip it for now
        Ok(())
    }

    #[sqlx::test]
    async fn test_author_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let privy_id = create_test_user(&sql_client, "author").await?;

        let author = create_test_author(&sql_client, &privy_id).await?;
        assert_eq!(author.name, format!("Test Author {}", privy_id));
        assert_eq!(
            author.email,
            Some(format!("author_{}@example.com", privy_id))
        );

        let retrieved_author = sql_client.get_author(&author.privy_id).await?;
        assert_eq!(retrieved_author.privy_id, author.privy_id);
        assert_eq!(retrieved_author.name, author.name);

        let result = sql_client
            .update_author(
                &author.privy_id,
                Some("Updated Author"),
                Some("updated@example.com"),
                Some("Updated University"),
                None, // wallet_address not being updated
            )
            .await?;
        assert!(result.rows_affected() > 0);

        let delete_result = sql_client.delete_author(&author.privy_id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_author_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        for i in 0..3 {
            let user_privy_id = create_test_user(&sql_client, &format!("author{}", i)).await?;
            create_test_author(&sql_client, &user_privy_id).await?;
        }

        let authors = sql_client.list_authors(Some(1), Some(10)).await?;
        assert_eq!(authors.len(), 3);

        let count = sql_client.count_authors().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let user1_privy_id = create_test_user(&sql_client, "user1").await?;
        let user2_privy_id = create_test_user(&sql_client, "user2").await?;

        let pub1 =
            create_test_publication(&sql_client, &user1_privy_id, Some("Citing Publication"))
                .await?;
        let pub2 = create_test_publication(&sql_client, &user2_privy_id, Some("Cited Publication"))
            .await?;

        let new_citation = NewCitation {
            citing_publication_id: pub1.id,
            cited_publication_id: pub2.id,
        };

        let citation = sql_client.create_citation(&new_citation).await?;
        assert_eq!(citation.citing_publication_id, pub1.id);
        assert_eq!(citation.cited_publication_id, pub2.id);

        let retrieved_citation = sql_client.get_citation(citation.id).await?;
        assert_eq!(retrieved_citation.id, citation.id);

        let citation_by_pubs = sql_client
            .get_citation_by_publications(pub1.id, pub2.id)
            .await?;
        assert!(citation_by_pubs.is_some());
        assert_eq!(citation_by_pubs.unwrap().id, citation.id);

        let result = sql_client.update_citation(citation.id).await?;
        assert!(result.rows_affected() == 0); // No fields to update

        let delete_result = sql_client.delete_citation(citation.id).await?;
        assert!(delete_result.rows_affected() > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_listing(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let mut pub_results = Vec::new();
        for i in 0..4 {
            let user_privy_id = create_test_user(&sql_client, &format!("user{}", i)).await?;
            let publication = create_test_publication(
                &sql_client,
                &user_privy_id,
                Some(&format!("Publication {}", i)),
            )
            .await?;
            pub_results.push(publication);
        }

        for i in 0..3 {
            let new_citation = NewCitation {
                citing_publication_id: pub_results[i].id,
                cited_publication_id: pub_results[i + 1].id,
            };
            sql_client.create_citation(&new_citation).await?;
        }

        let citations = sql_client.list_citations(Some(1), Some(10)).await?;
        assert_eq!(citations.len(), 3);

        let count = sql_client.count_citations().await?;
        assert_eq!(count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_author_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let pub_user_privy_id = create_test_user(&sql_client, "pub_user").await?;
        let author_user_privy_id = create_test_user(&sql_client, "author_user").await?;

        let publication =
            create_test_publication(&sql_client, &pub_user_privy_id, Some("Test Publication"))
                .await?;
        let author = create_test_author(&sql_client, &author_user_privy_id).await?;

        sql_client
            .add_author_to_publication(publication.id, &author.privy_id, Some(1))
            .await?;

        let pub_authors =
            PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id)
                .await?;
        assert_eq!(pub_authors.len(), 1);
        assert_eq!(pub_authors[0].author_id, author.privy_id);

        let has_author = sql_client
            .publication_has_author(publication.id, &author.privy_id)
            .await?;
        assert!(has_author);

        let author_count = sql_client
            .count_authors_for_publication(publication.id)
            .await?;
        assert_eq!(author_count, 1);

        let remove_result = sql_client
            .remove_author_from_publication(publication.id, &author.privy_id)
            .await?;
        assert!(remove_result.rows_affected() > 0);

        let pub_authors_after =
            PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id)
                .await?;
        assert!(pub_authors_after.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_set_publication_authors(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let pub_user_privy_id = create_test_user(&sql_client, "pub_user").await?;

        let publication =
            create_test_publication(&sql_client, &pub_user_privy_id, Some("Test Publication"))
                .await?;

        let mut author_results = Vec::new();
        for i in 0..3 {
            let author_user_privy_id =
                create_test_user(&sql_client, &format!("author{}", i)).await?;
            let author = create_test_author(&sql_client, &author_user_privy_id).await?;
            author_results.push(author);
        }

        let author_privy_ids: Vec<String> =
            author_results.iter().map(|a| a.privy_id.clone()).collect();
        sql_client
            .set_publication_authors(publication.id, &author_privy_ids)
            .await?;

        let pub_authors =
            PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id)
                .await?;
        assert_eq!(pub_authors.len(), 3);

        for (i, pub_author) in pub_authors.iter().enumerate() {
            assert_eq!(pub_author.author_id, author_privy_ids[i]);
            assert_eq!(pub_author.author_order, (i + 1) as i32);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_search_publications_by_title(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;
        let publications = vec![
            "Machine Learning Advances",
            "Deep Learning Research",
            "Artificial Intelligence Review",
            "Machine Vision Systems",
        ];

        for (index, title) in publications.iter().enumerate() {
            let user_privy_id =
                create_test_user(&sql_client, &format!("search_title{}", index)).await?;
            create_test_publication(&sql_client, &user_privy_id, Some(title)).await?;
        }

        let machine_pubs = sql_client
            .search_publications_by_title("Machine", Some(1), Some(10))
            .await?;

        assert_eq!(machine_pubs.len(), 2);
        for pub_item in &machine_pubs {
            assert!(pub_item.title.contains("Machine"));
        }

        let learning_pubs = sql_client
            .search_publications_by_title("Learning", Some(1), Some(10))
            .await?;

        assert_eq!(learning_pubs.len(), 2);
        for pub_item in &learning_pubs {
            assert!(pub_item.title.contains("Learning"));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_search_publications_by_tag(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let publications = vec![
            ("Paper 1", vec!["ai".to_string(), "ml".to_string()]),
            ("Paper 2", vec!["ml".to_string(), "dl".to_string()]),
            ("Paper 3", vec!["ai".to_string(), "cv".to_string()]),
            ("Paper 4", vec!["nlp".to_string()]),
        ];

        for (index, (title, tags)) in publications.iter().enumerate() {
            let user_privy_id =
                create_test_user(&sql_client, &format!("search_tag{}", index)).await?;
            let _publication = sql_client
                .create_publication(&NewPublication {
                    user_id: user_privy_id,
                    title: title.to_string(),
                    about: "Test description".to_string(),
                    tags: tags.clone(),
                    s3key: "".to_string(),
                    price: 0,
                    citation_royalty_bps: 0,
                })
                .await?;
        }

        let ai_pubs = sql_client
            .search_publications_by_tag("ai", Some(1), Some(10))
            .await?;

        assert_eq!(ai_pubs.len(), 2);
        for pub_item in &ai_pubs {
            assert!(pub_item.tags.contains(&"ai".to_string()));
        }

        let ml_pubs = sql_client
            .search_publications_by_tag("ml", Some(1), Some(10))
            .await?;

        assert_eq!(ml_pubs.len(), 2);
        for pub_item in &ml_pubs {
            assert!(pub_item.tags.contains(&"ml".to_string()));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_citation_relationships(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let mut publications = Vec::new();
        for i in 0..5 {
            // First create a user
            let new_user = NewUser {
                privy_id: format!("test_user_{}", i),
            };
            sql_client.create_user(&new_user).await?;

            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: format!("test_user_{}", i),
                    title: format!("Publication {}", i),
                    about: "".to_string(),
                    tags: vec![],
                    s3key: "".to_string(),
                    price: 0,
                    citation_royalty_bps: 0,
                })
                .await?;
            publications.push(publication);
        }

        // Create citation relationships: 0 cites 1, 1 cites 2, 2 cites 3, 3 cites 4
        for i in 0..4 {
            let new_citation = NewCitation {
                citing_publication_id: publications[i].id,
                cited_publication_id: publications[i + 1].id,
            };
            sql_client.create_citation(&new_citation).await?;
        }

        let pub1_citations = sql_client
            .get_publication_citations(publications[1].id)
            .await?;
        assert_eq!(pub1_citations.len(), 2);

        let cited_by = sql_client.get_cited_by(publications[2].id).await?;
        assert_eq!(cited_by.len(), 1);
        assert_eq!(cited_by[0].id, publications[1].id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_author_publications_relationship(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let new_user = NewUser {
            privy_id: "test_author_relationship".to_string(),
        };
        sql_client.create_user(&new_user).await?;

        let author =
            create_test_author(&sql_client, &"test_author_relationship".to_string()).await?;

        let mut publications = Vec::new();
        for i in 0..5 {
            let new_user = NewUser {
                privy_id: format!("test_user_{}", i),
            };
            sql_client.create_user(&new_user).await?;

            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: format!("test_user_{}", i),
                    title: format!("Publication {}", i),
                    about: "".to_string(),
                    tags: vec![],
                    s3key: "".to_string(),
                    price: 0,
                    citation_royalty_bps: 0,
                })
                .await?;
            publications.push(publication);
        }

        for i in 0..3 {
            sql_client
                .add_author_to_publication(publications[i].id, &author.privy_id, Some(i as i32))
                .await?;
        }

        let author_pubs = sql_client
            .get_author_publications(&author.privy_id, Some(1), Some(10))
            .await?;

        assert_eq!(author_pubs.len(), 3);

        let pub_count = sql_client
            .count_publications_for_author(&author.privy_id)
            .await?;
        assert_eq!(pub_count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_author_order(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let pub_user_privy_id = create_test_user(&sql_client, "pub_user_order").await?;
        let author_user_privy_id = create_test_user(&sql_client, "author_user_order").await?;

        let publication =
            create_test_publication(&sql_client, &pub_user_privy_id, Some("Test Publication"))
                .await?;
        let author = create_test_author(&sql_client, &author_user_privy_id).await?;

        sql_client
            .add_author_to_publication(publication.id, &author.privy_id, Some(1))
            .await?;

        let result = sql_client
            .update_author_order(publication.id, &author.privy_id, 2)
            .await?;
        assert!(result.rows_affected() > 0);

        let pub_authors =
            PublicationAuthorOperations::get_publication_authors(&sql_client, publication.id)
                .await?;
        assert_eq!(pub_authors.len(), 1);
        assert_eq!(pub_authors[0].author_order, 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_crud_operations(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let user_privy_id = create_test_user(&sql_client, "crud").await?;

        let new_publication = NewPublication {
            user_id: user_privy_id.clone(),
            title: "Test Publication".to_string(),
            about: "This is a test publication".to_string(),
            tags: vec!["test".to_string(), "ai".to_string()],
            s3key: "s3://bucket/key.pdf".to_string(),
            price: 0,
            citation_royalty_bps: 0,
        };

        let publication = sql_client.create_publication(&new_publication).await?;
        assert_eq!(publication.title, "Test Publication");
        assert_eq!(
            publication.about,
            "This is a test publication".to_string()
        );
        assert_eq!(publication.tags, vec!["test".to_string(), "ai".to_string()]);
        assert_eq!(publication.s3key, "s3://bucket/key.pdf".to_string());

        let retrieved_publication = sql_client.get_publication(publication.id).await?;
        assert_eq!(retrieved_publication.id, publication.id);
        assert_eq!(retrieved_publication.title, "Test Publication");

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

        let updated_publication = sql_client.get_publication(publication.id).await?;
        assert_eq!(updated_publication.title, "Updated Title");
        assert_eq!(
            updated_publication.about,
            "Updated description".to_string()
        );
        assert_eq!(
            updated_publication.tags,
            vec!["updated".to_string(), "ml".to_string()]
        );
        assert_eq!(
            updated_publication.s3key,
            "s3://bucket/updated.pdf".to_string()
        );

        let delete_result = sql_client.delete_publication(publication.id).await?;
        assert!(delete_result.rows_affected() > 0);

        let deleted_result = sql_client.get_publication(publication.id).await;
        assert!(deleted_result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_publications_by_user(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let user_privy_id = create_test_user(&sql_client, "main_user").await?;

        for i in 0..3 {
            create_test_publication(
                &sql_client,
                &user_privy_id,
                Some(&format!("User Publication {}", i)),
            )
            .await?;
        }

        for i in 0..2 {
            let diff_user_privy_id =
                create_test_user(&sql_client, &format!("different_user{}", i)).await?;
            sql_client
                .create_publication(&NewPublication {
                    user_id: diff_user_privy_id,
                    title: format!("Anonymous Publication {}", i),
                    about: "".to_string(),
                    tags: vec![],
                    s3key: "".to_string(),
                    price: 0,
                    citation_royalty_bps: 0,
                })
                .await?;
        }

        let user_publications = sql_client
            .list_publications_by_user(&user_privy_id, Some(1), Some(10))
            .await?;

        assert_eq!(user_publications.len(), 3);
        for pub_item in &user_publications {
            assert_eq!(pub_item.user_id, user_privy_id.clone());
            assert!(pub_item.title.starts_with("User Publication"));
        }

        let user_pub_count = sql_client
            .count_publications_by_user(&user_privy_id)
            .await?;
        assert_eq!(user_pub_count, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_and_count_publications(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        for i in 0..5 {
            let user_privy_id = create_test_user(&sql_client, &format!("list{}", i)).await?;
            create_test_publication(
                &sql_client,
                &user_privy_id,
                Some(&format!("Publication {}", i)),
            )
            .await?;
        }

        let page1 = sql_client.list_publications(Some(1), Some(3)).await?;
        assert_eq!(page1.len(), 3);

        let page2 = sql_client.list_publications(Some(2), Some(3)).await?;
        assert_eq!(page2.len(), 2);

        let total_count = sql_client.count_publications().await?;
        assert_eq!(total_count, 5);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_pagination_edge_cases(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        for i in 0..25 {
            let user_privy_id = create_test_user(&sql_client, &format!("pagination{}", i)).await?;
            create_test_publication(
                &sql_client,
                &user_privy_id,
                Some(&format!("Publication {}", i)),
            )
            .await?;
        }

        let default_page = sql_client.list_publications(None, None).await?;
        assert_eq!(default_page.len(), 20);

        let empty_page = sql_client.list_publications(Some(10), Some(10)).await?;
        assert!(empty_page.is_empty());

        let small_page = sql_client.list_publications(Some(1), Some(5)).await?;
        assert_eq!(small_page.len(), 5);

        let large_page = sql_client.list_publications(Some(1), Some(100)).await?;
        assert_eq!(large_page.len(), 25);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publication_update_partial_fields(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let sql_client = SqlClient::new(pool.clone()).await;

        let user_privy_id = create_test_user(&sql_client, "update").await?;

        let publication = sql_client
            .create_publication(&NewPublication {
                user_id: user_privy_id.clone(),
                title: "Original Title".to_string(),
                about: "Original description".to_string(),
                tags: vec!["original".to_string()],
                s3key: "s3://original.pdf".to_string(),
                price: 0,
                citation_royalty_bps: 0,
            })
            .await?;

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
        assert_eq!(
            after_title_update.about,
            "Original description".to_string()
        );
        assert_eq!(after_title_update.tags, vec!["original".to_string()]);
        assert_eq!(
            after_title_update.s3key,
            "s3://original.pdf".to_string()
        );

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
        assert_eq!(
            after_tags_update.tags,
            vec!["updated".to_string(), "tags".to_string()]
        );

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
        assert_eq!(
            after_s3key_update.s3key,
            "s3://updated.pdf".to_string()
        );

        Ok(())
    }
}
