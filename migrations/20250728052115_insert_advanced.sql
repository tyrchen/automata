-- Add migration script here
-- Advanced demo workflows showcasing complex patterns
-- Demo 5: Multi-step Approval Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('e5f6a7b8-c9d0-1234-efab-567890123456', 'Document Approval Workflow', 'Multi-level approval process with notifications and escalation', 'metadata:
  name: "Document Approval Workflow"
  version: "1.0.0"
  description: "Multi-level approval process with notifications and escalation"
  tags: ["approval", "document", "multi-step"]

triggers:
  - webhook:
      path: "/api/documents/submit"
      method: "POST"

nodes:
  validate_document:
    type: validator
    rules:
      - field: document_id
        type: string
        required: true
      - field: document_type
        type: string
        required: true
        allowed_values: ["contract", "proposal", "policy", "report"]
      - field: submitter_id
        type: string
        required: true
      - field: department
        type: string
        required: true

  enrich_document:
    type: transformer
    mapping:
      document_id: $validate_document.data.document_id
      document_type: $validate_document.data.document_type
      submitter_id: $validate_document.data.submitter_id
      department: $validate_document.data.department
      submission_date: $now()
      approval_chain: $getApprovalChain($validate_document.data.document_type, $validate_document.data.department)
      status: "pending_approval"
      current_level: 0

  get_approvers:
    type: database
    operation: query
    query: "SELECT * FROM approval_matrix WHERE document_type = $1 AND department = $2 ORDER BY level"
    params:
      - $enrich_document.output.document_type
      - $enrich_document.output.department

  notify_approvers:
    type: forEach
    items: $get_approvers.output
    operation:
      type: parallel
      branches:
        - send_email:
            type: http
            method: POST
            url: "https://api.email.com/send"
            body:
              to: $item.approver_email
              subject: "Document Pending Approval: $enrich_document.output.document_id"
              template: "approval_request"
              data:
                document_id: $enrich_document.output.document_id
                document_type: $enrich_document.output.document_type
                submitter: $enrich_document.output.submitter_id
                level: $item.level
        - send_slack:
            type: http
            method: POST
            url: "https://slack.com/api/chat.postMessage"
            headers:
              Authorization: "Bearer $env(SLACK_TOKEN)"
            body:
              channel: $item.slack_channel
              text: "New document awaiting approval"
              blocks:
                - type: section
                  text:
                    type: mrkdwn
                    text: "*Document ID:* $enrich_document.output.document_id\n*Type:* $enrich_document.output.document_type\n*Level:* $item.level"
                - type: actions
                  elements:
                    - type: button
                      text:
                        type: plain_text
                        text: "Approve"
                      value: "approve_$enrich_document.output.document_id"
                    - type: button
                      text:
                        type: plain_text
                        text: "Reject"
                      value: "reject_$enrich_document.output.document_id"

  wait_for_approval:
    type: event
    event_type: "approval.decision"
    timeout: 86400
    filter: "data.document_id == $enrich_document.output.document_id"

  process_decision:
    type: conditional
    condition: $wait_for_approval.data.decision == "approved"
    then:
      type: transformer
      mapping:
        document_id: $enrich_document.output.document_id
        current_level: $enrich_document.output.current_level + 1
        approved_by: $wait_for_approval.data.approver_id
        approved_at: $now()
        next_action: $if($enrich_document.output.current_level < $len($get_approvers.output) - 1, "next_level", "complete")
    else:
      type: transformer
      mapping:
        document_id: $enrich_document.output.document_id
        rejected_by: $wait_for_approval.data.approver_id
        rejected_at: $now()
        reason: $wait_for_approval.data.reason
        next_action: "rejected"

  route_next:
    type: switch
    expression: $process_decision.output.next_action
    cases:
      - when: "next_level"
        then:
          type: transformer
          mapping:
            restart_approval: true
            document_id: $enrich_document.output.document_id
            current_level: $process_decision.output.current_level
      - when: "complete"
        then:
          type: database
          operation: update
          table: documents
          condition: "id = $enrich_document.output.document_id"
          data:
            status: "approved"
            approved_at: $now()
            approval_history: $process_decision.output
      - when: "rejected"
        then:
          type: database
          operation: update
          table: documents
          condition: "id = $enrich_document.output.document_id"
          data:
            status: "rejected"
            rejected_at: $now()
            rejection_reason: $process_decision.output.reason

connections:
  - from: trigger
    to: validate_document
  - from: validate_document
    to: enrich_document
    condition: $validate_document.success
  - from: enrich_document
    to: get_approvers
  - from: get_approvers
    to: notify_approvers
  - from: notify_approvers
    to: wait_for_approval
  - from: wait_for_approval
    to: process_decision
  - from: process_decision
    to: route_next', '1.0.0', 'active');

-- Demo 6: Data Synchronization Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('f6a7b8c9-d0e1-2345-fabc-678901234567', 'Cross-System Data Sync', 'Synchronize data between multiple systems with conflict resolution', 'metadata:
  name: "Cross-System Data Sync"
  version: "1.0.0"
  description: "Synchronize data between multiple systems with conflict resolution"
  tags: ["sync", "integration", "etl"]

triggers:
  - schedule:
      cron: "*/15 * * * *"
      timezone: "UTC"
  - event:
      source: "data-change"
      event_type: "entity.modified"

nodes:
  identify_changes:
    type: parallel
    branches:
      - check_system_a:
          type: http
          method: GET
          url: "https://system-a.example.com/api/changes"
          headers:
            Authorization: "Bearer $env(SYSTEM_A_TOKEN)"
          params:
            since: $lastSync()
            limit: 1000
      - check_system_b:
          type: http
          method: GET
          url: "https://system-b.example.com/api/changes"
          headers:
            X-API-Key: "$env(SYSTEM_B_KEY)"
          params:
            modified_after: $lastSync()
      - check_database:
          type: database
          operation: query
          query: "SELECT * FROM entities WHERE updated_at > $1"
          params:
            - $lastSync()

  merge_changes:
    type: transformer
    mapping:
      all_changes: $flatten([$identify_changes.check_system_a.output.items, $identify_changes.check_system_b.output.records, $identify_changes.check_database.output])
      unique_entities: $unique($all_changes, "entity_id")
      conflict_groups: $groupBy($all_changes, "entity_id")
      has_conflicts: $any($conflict_groups, $len($item) > 1)

  resolve_conflicts:
    type: forEach
    items: $filter($merge_changes.output.conflict_groups, $len($item) > 1)
    operation:
      type: transformer
      mapping:
        entity_id: $item[0].entity_id
        winner: $maxBy($item, "updated_at")
        resolution_strategy: $if($item[0].priority == "high", "manual", "last_write_wins")
        conflict_data:
          sources: $map($item, $object("system", $item.source, "version", $item.version))
          differences: $diff($item[0], $item[1])

  prepare_sync_batches:
    type: transformer
    mapping:
      system_a_updates: $filter($merge_changes.output.unique_entities, $item.source != "system_a")
      system_b_updates: $filter($merge_changes.output.unique_entities, $item.source != "system_b")
      database_updates: $filter($merge_changes.output.unique_entities, $item.source != "database")
      total_updates: $len($merge_changes.output.unique_entities)
      batch_size: 100

  sync_to_systems:
    type: parallel
    branches:
      - sync_system_a:
          type: forEach
          items: $chunk($prepare_sync_batches.output.system_a_updates, $prepare_sync_batches.output.batch_size)
          operation:
            type: http
            method: POST
            url: "https://system-a.example.com/api/bulk-update"
            retry:
              max_attempts: 3
              backoff_multiplier: 2
            headers:
              Authorization: "Bearer $env(SYSTEM_A_TOKEN)"
            body:
              entities: $item
              sync_id: $uuid()
      - sync_system_b:
          type: forEach
          items: $chunk($prepare_sync_batches.output.system_b_updates, $prepare_sync_batches.output.batch_size)
          operation:
            type: http
            method: PUT
            url: "https://system-b.example.com/api/batch"
            headers:
              X-API-Key: "$env(SYSTEM_B_KEY)"
            body:
              batch: $item
              timestamp: $now()
      - sync_database:
          type: forEach
          items: $prepare_sync_batches.output.database_updates
          operation:
            type: database
            operation: upsert
            table: entities
            key: entity_id
            data: $item

  verify_sync:
    type: parallel
    branches:
      - verify_counts:
          type: transformer
          mapping:
            expected_total: $prepare_sync_batches.output.total_updates * 3
            actual_synced: $sum([$len($sync_to_systems.sync_system_a.output), $len($sync_to_systems.sync_system_b.output), $len($sync_to_systems.sync_database.output)])
            success_rate: $actual_synced / $expected_total
      - check_errors:
          type: transformer
          mapping:
            system_a_errors: $filter($sync_to_systems.sync_system_a.output, $item.error)
            system_b_errors: $filter($sync_to_systems.sync_system_b.output, $item.error)
            database_errors: $filter($sync_to_systems.sync_database.output, $item.error)
            total_errors: $len($system_a_errors) + $len($system_b_errors) + $len($database_errors)

  record_sync_result:
    type: database
    operation: insert
    table: sync_history
    data:
      sync_id: $uuid()
      started_at: $trigger.timestamp
      completed_at: $now()
      total_entities: $prepare_sync_batches.output.total_updates
      conflicts_resolved: $len($resolve_conflicts.output)
      success_rate: $verify_sync.verify_counts.output.success_rate
      errors: $verify_sync.check_errors.output
      status: $if($verify_sync.check_errors.output.total_errors > 0, "completed_with_errors", "success")

  send_sync_report:
    type: conditional
    condition: $verify_sync.check_errors.output.total_errors > 0 || $prepare_sync_batches.output.total_updates > 1000
    then:
      type: http
      method: POST
      url: "https://api.notifications.com/alert"
      body:
        channel: "data-ops"
        severity: $if($verify_sync.check_errors.output.total_errors > 0, "warning", "info")
        title: "Data Sync Report"
        message: "Synced $prepare_sync_batches.output.total_updates entities with $verify_sync.check_errors.output.total_errors errors"
        details: $record_sync_result.output

connections:
  - from: trigger
    to: identify_changes
  - from: identify_changes
    to: merge_changes
  - from: merge_changes
    to: resolve_conflicts
    condition: $merge_changes.output.has_conflicts
  - from: merge_changes
    to: prepare_sync_batches
  - from: resolve_conflicts
    to: prepare_sync_batches
  - from: prepare_sync_batches
    to: sync_to_systems
  - from: sync_to_systems
    to: verify_sync
  - from: verify_sync
    to: record_sync_result
  - from: record_sync_result
    to: send_sync_report', '1.0.0', 'active');

-- Demo 7: ML Pipeline Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('a7b8c9d0-e1f2-3456-abcd-789012345678', 'ML Model Training Pipeline', 'Automated machine learning model training with data preprocessing and evaluation', 'metadata:
  name: "ML Model Training Pipeline"
  version: "1.0.0"
  description: "Automated machine learning model training with data preprocessing and evaluation"
  tags: ["ml", "ai", "training", "pipeline"]

triggers:
  - schedule:
      cron: "0 0 * * 0"
      timezone: "UTC"
  - event:
      source: "ml-platform"
      event_type: "dataset.ready"

nodes:
  load_dataset:
    type: http
    method: GET
    url: "https://data-lake.example.com/datasets/$trigger.data.dataset_id"
    headers:
      Authorization: "Bearer $env(DATA_LAKE_TOKEN)"
    timeout: 300
    retry:
      max_attempts: 3

  validate_dataset:
    type: validator
    rules:
      - field: features
        type: array
        required: true
        min_items: 1
      - field: labels
        type: array
        required: true
        min_items: 100
      - field: metadata.version
        type: string
        required: true

  preprocess_data:
    type: http
    method: POST
    url: "https://ml-service.example.com/preprocess"
    timeout: 600
    body:
      dataset_id: $trigger.data.dataset_id
      features: $load_dataset.output.features
      preprocessing_steps:
        - type: "normalize"
          method: "min_max"
        - type: "handle_missing"
          strategy: "median"
        - type: "encode_categorical"
          method: "one_hot"
        - type: "feature_selection"
          method: "mutual_information"
          top_k: 50

  split_data:
    type: transformer
    mapping:
      train_size: $floor($len($preprocess_data.output.processed_features) * 0.7)
      val_size: $floor($len($preprocess_data.output.processed_features) * 0.15)
      test_size: $len($preprocess_data.output.processed_features) - $train_size - $val_size
      train_data:
        features: $slice($preprocess_data.output.processed_features, 0, $train_size)
        labels: $slice($load_dataset.output.labels, 0, $train_size)
      val_data:
        features: $slice($preprocess_data.output.processed_features, $train_size, $train_size + $val_size)
        labels: $slice($load_dataset.output.labels, $train_size, $train_size + $val_size)
      test_data:
        features: $slice($preprocess_data.output.processed_features, $train_size + $val_size)
        labels: $slice($load_dataset.output.labels, $train_size + $val_size)

  train_models:
    type: parallel
    branches:
      - train_rf:
          type: http
          method: POST
          url: "https://ml-service.example.com/train"
          timeout: 3600
          body:
            algorithm: "random_forest"
            hyperparameters:
              n_estimators: 100
              max_depth: 10
              min_samples_split: 5
            train_data: $split_data.output.train_data
            val_data: $split_data.output.val_data
      - train_xgb:
          type: http
          method: POST
          url: "https://ml-service.example.com/train"
          timeout: 3600
          body:
            algorithm: "xgboost"
            hyperparameters:
              learning_rate: 0.1
              n_estimators: 100
              max_depth: 6
            train_data: $split_data.output.train_data
            val_data: $split_data.output.val_data
      - train_nn:
          type: http
          method: POST
          url: "https://ml-service.example.com/train"
          timeout: 7200
          body:
            algorithm: "neural_network"
            architecture:
              layers: [64, 32, 16, 1]
              activation: "relu"
              dropout: 0.2
            training_params:
              epochs: 100
              batch_size: 32
              early_stopping: true
            train_data: $split_data.output.train_data
            val_data: $split_data.output.val_data

  evaluate_models:
    type: forEach
    items: [$train_models.train_rf.output, $train_models.train_xgb.output, $train_models.train_nn.output]
    operation:
      type: http
      method: POST
      url: "https://ml-service.example.com/evaluate"
      body:
        model_id: $item.model_id
        test_data: $split_data.output.test_data
        metrics: ["accuracy", "precision", "recall", "f1", "auc_roc"]

  select_best_model:
    type: transformer
    mapping:
      models_with_scores: $map($evaluate_models.output, $object("model_id", $item.model_id, "algorithm", $item.algorithm, "f1_score", $item.metrics.f1))
      best_model: $maxBy($models_with_scores, "f1_score")
      improvement: ($best_model.f1_score - $getPreviousBestScore()) / $getPreviousBestScore()
      should_deploy: $best_model.f1_score > 0.85 && $improvement > 0.02

  deploy_model:
    type: conditional
    condition: $select_best_model.output.should_deploy
    then:
      type: parallel
      branches:
        - register_model:
            type: http
            method: POST
            url: "https://ml-service.example.com/models/register"
            body:
              model_id: $select_best_model.output.best_model.model_id
              name: "production_model_v$version()"
              metadata:
                algorithm: $select_best_model.output.best_model.algorithm
                f1_score: $select_best_model.output.best_model.f1_score
                training_date: $now()
                dataset_version: $load_dataset.output.metadata.version
        - update_api:
            type: http
            method: PUT
            url: "https://api.example.com/ml/models/active"
            headers:
              Authorization: "Bearer $env(API_TOKEN)"
            body:
              model_id: $select_best_model.output.best_model.model_id
              rollout_percentage: 10
        - notify_team:
            type: http
            method: POST
            url: "https://slack.com/api/chat.postMessage"
            headers:
              Authorization: "Bearer $env(SLACK_TOKEN)"
            body:
              channel: "#ml-team"
              text: "New model deployed! 🎉"
              blocks:
                - type: section
                  text:
                    type: mrkdwn
                    text: "*New Model Deployed*\n*Algorithm:* $select_best_model.output.best_model.algorithm\n*F1 Score:* $select_best_model.output.best_model.f1_score\n*Improvement:* $round($select_best_model.output.improvement * 100, 2)%"
    else:
      type: database
      operation: insert
      table: ml_training_logs
      data:
        reason: "Model did not meet deployment criteria"
        best_score: $select_best_model.output.best_model.f1_score
        threshold_score: 0.85
        improvement: $select_best_model.output.improvement

  cleanup:
    type: parallel
    branches:
      - archive_artifacts:
          type: http
          method: POST
          url: "https://storage.example.com/archive"
          body:
            artifacts: $map($train_models.output, $item.artifacts)
            retention_days: 30
      - update_metrics:
          type: database
          operation: insert
          table: ml_metrics
          data:
            training_id: $uuid()
            dataset_id: $trigger.data.dataset_id
            models_trained: 3
            best_model: $select_best_model.output.best_model
            deployed: $select_best_model.output.should_deploy
            completed_at: $now()

connections:
  - from: trigger
    to: load_dataset
  - from: load_dataset
    to: validate_dataset
  - from: validate_dataset
    to: preprocess_data
    condition: $validate_dataset.success
  - from: preprocess_data
    to: split_data
  - from: split_data
    to: train_models
  - from: train_models
    to: evaluate_models
  - from: evaluate_models
    to: select_best_model
  - from: select_best_model
    to: deploy_model
  - from: deploy_model
    to: cleanup
  - from: select_best_model
    to: cleanup
    condition: !$select_best_model.output.should_deploy', '1.0.0', 'active');

-- Demo 8: Incident Response Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('b8c9d0e1-f2a3-4567-bcde-890123456789', 'Automated Incident Response', 'Detect, triage, and respond to system incidents automatically', 'metadata:
  name: "Automated Incident Response"
  version: "1.0.0"
  description: "Detect, triage, and respond to system incidents automatically"
  tags: ["incident", "response", "automation", "ops"]

triggers:
  - event:
      source: "monitoring"
      event_type: "alert.triggered"
  - webhook:
      path: "/api/incidents/report"
      method: "POST"

nodes:
  enrich_incident:
    type: parallel
    branches:
      - get_metrics:
          type: http
          method: GET
          url: "https://metrics.example.com/api/query"
          params:
            service: $trigger.data.service
            time_range: "15m"
            metrics: ["cpu", "memory", "requests", "errors", "latency"]
      - get_logs:
          type: http
          method: POST
          url: "https://logs.example.com/search"
          body:
            service: $trigger.data.service
            level: "error"
            time_range:
              from: $now() - 900000
              to: $now()
            limit: 100
      - get_dependencies:
          type: database
          operation: query
          query: "SELECT * FROM service_dependencies WHERE service_name = $1"
          params:
            - $trigger.data.service

  analyze_severity:
    type: transformer
    mapping:
      service: $trigger.data.service
      alert_type: $trigger.data.alert_type
      metrics_anomaly_score: $calculateAnomalyScore($enrich_incident.get_metrics.output)
      error_rate: $enrich_incident.get_metrics.output.errors / $enrich_incident.get_metrics.output.requests
      has_customer_impact: $any($enrich_incident.get_dependencies.output, $item.is_customer_facing)
      similar_past_incidents: $searchSimilarIncidents($trigger.data)
      severity_score: $calculateSeverity({
        "error_rate": $error_rate,
        "customer_impact": $has_customer_impact,
        "anomaly_score": $metrics_anomaly_score,
        "alert_priority": $trigger.data.priority
      })
      severity_level: $if($severity_score > 0.8, "critical", $if($severity_score > 0.6, "high", $if($severity_score > 0.4, "medium", "low")))

  create_incident:
    type: database
    operation: insert
    table: incidents
    data:
      incident_id: $uuid()
      service: $analyze_severity.output.service
      severity: $analyze_severity.output.severity_level
      title: "$trigger.data.alert_type in $trigger.data.service"
      description: $trigger.data.description
      metrics: $enrich_incident.get_metrics.output
      logs_sample: $slice($enrich_incident.get_logs.output, 0, 10)
      created_at: $now()
      status: "open"
      assigned_to: $null()

  route_response:
    type: switch
    expression: $analyze_severity.output.severity_level
    cases:
      - when: "critical"
        then:
          type: parallel
          branches:
            - page_oncall:
                type: http
                method: POST
                url: "https://pagerduty.com/api/incidents"
                headers:
                  Authorization: "Token token=$env(PAGERDUTY_TOKEN)"
                body:
                  incident:
                    type: "incident"
                    title: "CRITICAL: $create_incident.output.title"
                    service:
                      id: "$env(PAGERDUTY_SERVICE_ID)"
                    urgency: "high"
                    body:
                      type: "incident_body"
                      details: $create_incident.output
            - auto_scale:
                type: http
                method: POST
                url: "https://cloud.example.com/api/autoscale"
                headers:
                  Authorization: "Bearer $env(CLOUD_API_TOKEN)"
                body:
                  service: $trigger.data.service
                  action: "scale_up"
                  instances: 2
            - enable_circuit_breaker:
                type: http
                method: PUT
                url: "https://api.example.com/services/$trigger.data.service/circuit-breaker"
                body:
                  enabled: true
                  threshold: 0.5
      - when: "high"
        then:
          type: parallel
          branches:
            - notify_team:
                type: http
                method: POST
                url: "https://slack.com/api/chat.postMessage"
                headers:
                  Authorization: "Bearer $env(SLACK_TOKEN)"
                body:
                  channel: "#ops-alerts"
                  text: "High severity incident detected"
                  attachments:
                    - color: "warning"
                      title: $create_incident.output.title
                      fields:
                        - title: "Service"
                          value: $trigger.data.service
                          short: true
                        - title: "Error Rate"
                          value: "$round($analyze_severity.output.error_rate * 100, 2)%"
                          short: true
            - create_jira:
                type: http
                method: POST
                url: "https://jira.example.com/rest/api/2/issue"
                auth:
                  type: "basic"
                  username: "$env(JIRA_USER)"
                  password: "$env(JIRA_TOKEN)"
                body:
                  fields:
                    project:
                      key: "OPS"
                    summary: $create_incident.output.title
                    description: $create_incident.output.description
                    issuetype:
                      name: "Incident"
                    priority:
                      name: "High"
      - default:
          type: database
          operation: update
          table: incidents
          condition: "incident_id = $create_incident.output.incident_id"
          data:
            auto_resolved: true
            resolved_at: $now()
            resolution: "Low severity - monitoring only"

  attempt_auto_remediation:
    type: conditional
    condition: $analyze_severity.output.similar_past_incidents && $analyze_severity.output.severity_level != "critical"
    then:
      type: forEach
      items: $analyze_severity.output.similar_past_incidents[0].remediation_steps
      operation:
        type: switch
        expression: $item.action
        cases:
          - when: "restart_service"
            then:
              type: http
              method: POST
              url: "https://orchestrator.example.com/services/$trigger.data.service/restart"
              headers:
                Authorization: "Bearer $env(ORCHESTRATOR_TOKEN)"
          - when: "clear_cache"
            then:
              type: http
              method: DELETE
              url: "https://cache.example.com/api/flush"
              params:
                service: $trigger.data.service
          - when: "rollback"
            then:
              type: http
              method: POST
              url: "https://deploy.example.com/rollback"
              body:
                service: $trigger.data.service
                version: $getPreviousStableVersion($trigger.data.service)

  monitor_resolution:
    type: event
    event_type: "incident.resolved"
    timeout: 3600
    filter: "data.incident_id == $create_incident.output.incident_id"

  update_runbook:
    type: conditional
    condition: $monitor_resolution.success && !$analyze_severity.output.similar_past_incidents
    then:
      type: database
      operation: insert
      table: incident_runbooks
      data:
        incident_type: $trigger.data.alert_type
        service: $trigger.data.service
        symptoms: $enrich_incident.output
        resolution_steps: $monitor_resolution.data.resolution_steps
        resolution_time: $monitor_resolution.data.resolved_at - $create_incident.output.created_at
        created_at: $now()

connections:
  - from: trigger
    to: enrich_incident
  - from: enrich_incident
    to: analyze_severity
  - from: analyze_severity
    to: create_incident
  - from: create_incident
    to: route_response
  - from: route_response
    to: attempt_auto_remediation
  - from: attempt_auto_remediation
    to: monitor_resolution
  - from: monitor_resolution
    to: update_runbook', '1.0.0', 'active');

-- Add more execution records for the new workflows
INSERT INTO executions(workflow_id, status, trigger_data, outputs, started_at, completed_at)
  VALUES ('e5f6a7b8-c9d0-1234-efab-567890123456', 'Running', '{"document_id": "DOC-2024-001", "document_type": "contract", "submitter_id": "user123", "department": "legal"}', '{"current_level": 1, "status": "awaiting_approval"}', NOW() - INTERVAL '30 minutes', NULL),
('f6a7b8c9-d0e1-2345-fabc-678901234567', 'Completed', '{"scheduled": true}', '{"total_synced": 1523, "conflicts_resolved": 12, "errors": 0}', NOW() - INTERVAL '3 hours', NOW() - INTERVAL '3 hours' + INTERVAL '5 minutes'),
('a7b8c9d0-e1f2-3456-abcd-789012345678', 'Completed', '{"dataset_id": "dataset-2024-01-ml", "source": "ml-platform"}', '{"best_model": "xgboost", "f1_score": 0.92, "deployed": true}', NOW() - INTERVAL '6 hours', NOW() - INTERVAL '4 hours'),
('b8c9d0e1-f2a3-4567-bcde-890123456789', 'Completed', '{"service": "payment-api", "alert_type": "high_error_rate", "priority": "high"}', '{"incident_id": "INC-2024-0142", "severity": "high", "auto_remediated": true}', NOW() - INTERVAL '45 minutes', NOW() - INTERVAL '30 minutes');
