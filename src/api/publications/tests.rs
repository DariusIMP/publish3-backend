#[cfg(test)]
mod tests {

    use actix_web::{http::StatusCode, test};
    use serde_json::json;
    use sqlx::PgPool;

    use crate::{
        api::tests::create_test_app,
        db::sql::{PublicationOperations, SqlClient, models::NewPublication},
    };

    /// Helper function to create multipart form body for publication create/update
    /// Returns (boundary, body_bytes) tuple
    fn create_publication_multipart_body(
        user_privy_id: Option<&str>,
        title: &str,
        about: Option<&str>,
        tags: Option<Vec<&str>>,
        include_file: bool,
    ) -> (String, Vec<u8>) {
        let boundary = "testboundary12345";
        let mut body = Vec::new();

        // Add userId field if provided
        if let Some(user_privy_id) = user_privy_id {
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"userId\"\r\n\r\n");
            body.extend_from_slice(user_privy_id.as_bytes());
            body.extend_from_slice(b"\r\n");
        }

        // Add title field
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"title\"\r\n\r\n");
        body.extend_from_slice(title.as_bytes());
        body.extend_from_slice(b"\r\n");

        // Add about field if provided
        if let Some(about) = about {
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"about\"\r\n\r\n");
            body.extend_from_slice(about.as_bytes());
            body.extend_from_slice(b"\r\n");
        }

        // Add tags field if provided (as JSON array)
        if let Some(tags) = tags {
            let tags_json = serde_json::to_string(&tags).unwrap();
            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(b"Content-Disposition: form-data; name=\"tags\"\r\n\r\n");
            body.extend_from_slice(tags_json.as_bytes());
            body.extend_from_slice(b"\r\n");
        }

        // Add file field if requested
        if include_file {
            // Create dummy PDF content
            let pdf_content = b"%PDF-1.4\n1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n2 0 obj\n<</Type/Pages/Kids[]/Count 0>>\nendobj\nxref\n0 3\n0000000000 65535 f \n0000000010 00000 n \n0000000053 00000 n \ntrailer\n<</Size 3/Root 1 0 R>>\nstartxref\n94\n%%EOF";

            body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
            body.extend_from_slice(
                b"Content-Disposition: form-data; name=\"file\"; filename=\"test.pdf\"\r\n",
            );
            body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
            body.extend_from_slice(pdf_content);
            body.extend_from_slice(b"\r\n");
        }

        // End boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        (boundary.to_string(), body)
    }

    #[sqlx::test]
    async fn test_create_publication_api(pool: PgPool) {
        // Setup
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        // Create multipart form body using helper function
        let (boundary, body) = create_publication_multipart_body(
            Some(&user_privy_id),
            "Test Publication via POST API",
            Some("This is a test publication created via POST API"),
            Some(vec!["test", "api", "post"]),
            false, // No file upload for this test
        );

        // Create request
        use actix_web::test::TestRequest;
        let req = TestRequest::post()
            .uri("/publications/create")
            .insert_header((
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            ))
            .insert_header(("Content-Length", body.len()))
            .set_payload(body)
            .to_request();

        // Call the service
        let resp = test::call_service(&app, req).await;
        let status = resp.status();

        assert_eq!(status, StatusCode::OK);

        // Success! The POST worked
        let body_json: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body_json["title"], "Test Publication via POST API");
        assert_eq!(
            body_json["about"],
            "This is a test publication created via POST API"
        );

        // Verify we can retrieve it via GET
        let publication_id = body_json["id"].as_str().unwrap();
        let get_req = test::TestRequest::get()
            .uri(&format!("/publications/{}", publication_id))
            .to_request();

        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn test_get_publication_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let new_publication = NewPublication {
            user_id: user_privy_id.clone(),
            title: "Test Get Publication".to_string(),
            about: Some("Test description".to_string()),
            tags: Some(vec!["test".to_string()]),
            s3key: None,
        };

        let publication = sql_client
            .create_publication(&new_publication)
            .await
            .unwrap();

        let req = test::TestRequest::get()
            .uri(&format!("/publications/{}", publication.id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["title"], "Test Get Publication");
        assert_eq!(body["id"], publication.id.to_string());
    }

    #[sqlx::test]
    async fn test_list_publications_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        for i in 0..3 {
            let new_publication = NewPublication {
                user_id: user_privy_id.clone(),
                title: format!("Publication {}", i),
                about: Some(format!("Description {}", i)),
                tags: Some(vec!["test".to_string()]),
                s3key: None,
            };
            sql_client
                .create_publication(&new_publication)
                .await
                .unwrap();
        }

        let req = test::TestRequest::get()
            .uri("/publications/list")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["publications"].is_array());
        let publications = body["publications"].as_array().unwrap();
        assert!(publications.len() >= 3);
        assert!(body["total"].as_i64().unwrap() >= 3);
    }

    #[sqlx::test]
    async fn test_update_publication_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        // Create a publication first
        let new_publication = NewPublication {
            user_id: user_privy_id.clone(),
            title: "Original Title".to_string(),
            about: Some("Original description".to_string()),
            tags: Some(vec!["original".to_string()]),
            s3key: None,
        };

        let publication = sql_client
            .create_publication(&new_publication)
            .await
            .unwrap();

        // Create multipart form body for update using helper function
        // Note: For update, all fields are optional
        let (boundary, body) = create_publication_multipart_body(
            None, // user_id not being updated
            "Updated Title",
            Some("Updated description"),
            Some(vec!["updated", "test"]),
            false, // No file upload for this test
        );

        // Create PUT request for update
        use actix_web::test::TestRequest;
        let req = TestRequest::put()
            .uri(&format!("/publications/{}", publication.id))
            .insert_header((
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            ))
            .insert_header(("Content-Length", body.len()))
            .set_payload(body)
            .to_request();

        // Call the service
        let resp = test::call_service(&app, req).await;
        let status = resp.status();

        assert_eq!(status, StatusCode::OK);

        // Verify the update was successful
        let body_json: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body_json["status"], "success");
        assert_eq!(body_json["message"], "Publication updated successfully");

        // Verify the publication was actually updated via GET
        let get_req = test::TestRequest::get()
            .uri(&format!("/publications/{}", publication.id))
            .to_request();

        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::OK);

        let get_body: serde_json::Value = test::read_body_json(get_resp).await;
        assert_eq!(get_body["title"], "Updated Title");
        assert_eq!(get_body["about"], "Updated description");

        // Check tags were updated
        let tags = get_body["tags"].as_array().unwrap();
        assert!(tags.contains(&json!("updated")));
        assert!(tags.contains(&json!("test")));
    }

    #[sqlx::test]
    async fn test_delete_publication_api(pool: PgPool) {
        // Setup
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        // Create test publication
        let new_publication = NewPublication {
            user_id: user_privy_id.clone(),
            title: "Publication to Delete".to_string(),
            about: Some("Will be deleted".to_string()),
            tags: Some(vec!["delete".to_string()]),
            s3key: None,
        };

        let publication = sql_client
            .create_publication(&new_publication)
            .await
            .unwrap();

        // Test DELETE request
        let req = test::TestRequest::delete()
            .uri(&format!("/publications/{}", publication.id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify deletion
        let get_req = test::TestRequest::get()
            .uri(&format!("/publications/{}", publication.id))
            .to_request();

        let get_resp = test::call_service(&app, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_search_publications_by_title_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let publications = vec![
            "Machine Learning Advances",
            "Deep Learning Research",
            "Artificial Intelligence Review",
        ];

        for title in publications {
            let new_publication = NewPublication {
                user_id: user_privy_id.clone(),
                title: title.to_string(),
                about: Some("Test description".to_string()),
                tags: Some(vec!["ai".to_string()]),
                s3key: None,
            };
            sql_client
                .create_publication(&new_publication)
                .await
                .unwrap();
        }

        let req = test::TestRequest::get()
            .uri("/publications/search/title?query=Learning")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        for publication in body {
            let title = publication["title"].as_str().unwrap();
            assert!(title.contains("Learning"));
        }
    }

    #[sqlx::test]
    async fn test_search_publications_by_tag_api(pool: PgPool) {
        let app = test::init_service(create_test_app(pool.clone()).await).await;
        let sql_client = SqlClient::new(pool).await;

        // Create test user first
        let user_privy_id = crate::api::tests::create_test_user(&sql_client).await;

        let publications = vec![
            ("Paper 1", vec!["ai".to_string(), "ml".to_string()]),
            ("Paper 2", vec!["ml".to_string(), "dl".to_string()]),
            ("Paper 3", vec!["ai".to_string(), "cv".to_string()]),
        ];

        for (title, tags) in publications {
            let new_publication = NewPublication {
                user_id: user_privy_id.clone(),
                title: title.to_string(),
                about: Some("Test description".to_string()),
                tags: Some(tags),
                s3key: None,
            };
            sql_client
                .create_publication(&new_publication)
                .await
                .unwrap();
        }

        let req = test::TestRequest::get()
            .uri("/publications/search/tag?tag=ai")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        for publication in body {
            let tags = publication["tags"].as_array().unwrap();
            assert!(tags.contains(&json!("ai")));
        }
    }
}
