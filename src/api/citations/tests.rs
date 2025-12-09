#[cfg(test)]
mod tests {

    use actix_web::{http::StatusCode, test};
    use serde_json::json;
    use sqlx::PgPool;

    use crate::{
        api::tests::create_test_app,
        db::sql::{
            CitationOperations, PublicationOperations, SqlClient,
            models::{NewCitation, NewPublication},
        },
    };

    #[sqlx::test]
    async fn test_create_citation_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test users first
        let user1_privy_id = crate::api::tests::create_test_user(&sql_client).await;
        let user2_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let pub1 = sql_client
            .create_publication(&NewPublication {
                user_id: user1_privy_id.clone(),
                title: "Citing Publication".to_string(),
                about: Some("This publication cites another".to_string()),
                tags: Some(vec!["citing".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let pub2 = sql_client
            .create_publication(&NewPublication {
                user_id: user2_privy_id.clone(),
                title: "Cited Publication".to_string(),
                about: Some("This publication is cited".to_string()),
                tags: Some(vec!["cited".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let request_body = json!({
            "citing_publication_id": pub1.id.to_string(),
            "cited_publication_id": pub2.id.to_string(),
        });

        let req = test::TestRequest::post()
            .uri("/citations/create")
            .set_json(&request_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["citing_publication_id"], pub1.id.to_string());
        assert_eq!(body["cited_publication_id"], pub2.id.to_string());
    }

    #[sqlx::test]
    async fn test_get_citation_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test users first
        let user1_privy_id = crate::api::tests::create_test_user(&sql_client).await;
        let user2_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let pub1 = sql_client
            .create_publication(&NewPublication {
                user_id: user1_privy_id.clone(),
                title: "Citing Publication".to_string(),
                about: Some("This publication cites another".to_string()),
                tags: Some(vec!["citing".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let pub2 = sql_client
            .create_publication(&NewPublication {
                user_id: user2_privy_id.clone(),
                title: "Cited Publication".to_string(),
                about: Some("This publication is cited".to_string()),
                tags: Some(vec!["cited".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let citation = sql_client
            .create_citation(&NewCitation {
                citing_publication_id: pub1.id,
                cited_publication_id: pub2.id,
            })
            .await
            .unwrap();

        let req = test::TestRequest::get()
            .uri(&format!("/citations/{}", citation.id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["id"], citation.id.to_string());
        assert_eq!(body["citing_publication_id"], pub1.id.to_string());
        assert_eq!(body["cited_publication_id"], pub2.id.to_string());
    }

    #[sqlx::test]
    async fn test_list_citations_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        let mut publications = Vec::new();
        for i in 0..4 {
            // Create test user for each publication
            let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;
            let publication = sql_client
                .create_publication(&NewPublication {
                    user_id: user_privy_id.clone(),
                    title: format!("Publication {}", i),
                    about: None,
                    tags: None,
                    s3key: None,
                })
                .await
                .unwrap();
            publications.push(publication);
        }

        for i in 0..3 {
            sql_client
                .create_citation(&NewCitation {
                    citing_publication_id: publications[i].id,
                    cited_publication_id: publications[i + 1].id,
                })
                .await
                .unwrap();
        }

        let req = test::TestRequest::get().uri("/citations/list").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["citations"].is_array());
        let citations = body["citations"].as_array().unwrap();
        assert!(citations.len() >= 3);
        assert!(body["total"].as_i64().unwrap() >= 3);
    }

    #[sqlx::test]
    async fn test_update_citation_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test users first
        let user1_privy_id = crate::api::tests::create_test_user(&sql_client).await;
        let user2_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let pub1 = sql_client
            .create_publication(&NewPublication {
                user_id: user1_privy_id.clone(),
                title: "Citing Publication".to_string(),
                about: Some("This publication cites another".to_string()),
                tags: Some(vec!["citing".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let pub2 = sql_client
            .create_publication(&NewPublication {
                user_id: user2_privy_id.clone(),
                title: "Cited Publication".to_string(),
                about: Some("This publication is cited".to_string()),
                tags: Some(vec!["cited".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let citation = sql_client
            .create_citation(&NewCitation {
                citing_publication_id: pub1.id,
                cited_publication_id: pub2.id,
            })
            .await
            .unwrap();

        let request_body = json!({});

        let req = test::TestRequest::put()
            .uri(&format!("/citations/{}", citation.id))
            .set_json(&request_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let get_req = test::TestRequest::get()
            .uri(&format!("/citations/{}", citation.id))
            .to_request();

        let get_resp = test::call_service(&app, get_req).await;
        let body: serde_json::Value = test::read_body_json(get_resp).await;
        // Verify citation exists
        assert_eq!(body["citing_publication_id"], pub1.id.to_string());
        assert_eq!(body["cited_publication_id"], pub2.id.to_string());
    }

    #[sqlx::test]
    async fn test_delete_citation_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test users first
        let user1_privy_id = crate::api::tests::create_test_user(&sql_client).await;
        let user2_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let pub1 = sql_client
            .create_publication(&NewPublication {
                user_id: user1_privy_id.clone(),
                title: "Citing Publication".to_string(),
                about: Some("This publication cites another".to_string()),
                tags: Some(vec!["citing".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let pub2 = sql_client
            .create_publication(&NewPublication {
                user_id: user2_privy_id.clone(),
                title: "Cited Publication".to_string(),
                about: Some("This publication is cited".to_string()),
                tags: Some(vec!["cited".to_string()]),
                s3key: None,
            })
            .await
            .unwrap();

        let citation = sql_client
            .create_citation(&NewCitation {
                citing_publication_id: pub1.id,
                cited_publication_id: pub2.id,
            })
            .await
            .unwrap();

        let req = test::TestRequest::delete()
            .uri(&format!("/citations/{}", citation.id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let get_req = test::TestRequest::get()
            .uri(&format!("/citations/{}", citation.id))
            .to_request();

        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
    }
}
