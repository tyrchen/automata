-- Add migration script here
-- Insert demo workflows for testing
-- Demo 1: Simple Hello World Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'Hello World Demo', 'A simple workflow that demonstrates basic functionality', 'metadata:
  name: "Hello World Demo"
  version: "1.0.0"
  description: "A simple workflow that demonstrates basic functionality"

triggers:
  - manual: {}

nodes:
  welcome:
    type: transformer
    mapping:
      message: "Hello, World!"
      timestamp: $now()
      workflow_name: "Hello World Demo"

  log_message:
    type: transformer
    mapping:
      log_entry: "Workflow executed at $welcome.output.timestamp"
      message: $welcome.output.message

connections:
  - from: trigger
    to: welcome
  - from: welcome
    to: log_message', '1.0.0', 'active');

-- Demo 2: User Registration Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('b2c3d4e5-f6a7-8901-bcde-f23456789012', 'User Registration Demo', 'Process new user registration with validation and email notification', 'metadata:
  name: "User Registration Demo"
  version: "1.0.0"
  description: "Process new user registration with validation and email notification"

triggers:
  - webhook:
      path: "/api/register"
      method: "POST"

nodes:
  validate_input:
    type: validator
    rules:
      - field: email
        type: string
        required: true
        pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
      - field: name
        type: string
        required: true
        min_length: 2
      - field: password
        type: string
        required: true
        min_length: 8

  transform_user:
    type: transformer
    mapping:
      user_id: $uuid()
      email: $validate_input.data.email
      name: $validate_input.data.name
      password_hash: $hash($validate_input.data.password, "bcrypt")
      created_at: $now()
      status: "pending_verification"

  save_user:
    type: database
    operation: insert
    table: users
    data: $transform_user.output

  send_welcome_email:
    type: http
    method: POST
    url: "https://api.example.com/email/send"
    headers:
      Content-Type: "application/json"
    body:
      to: $save_user.output.email
      subject: "Welcome to our platform!"
      template: "welcome"
      data:
        name: $save_user.output.name
        verification_link: "https://example.com/verify?token=$uuid()"

connections:
  - from: trigger
    to: validate_input
  - from: validate_input
    to: transform_user
    condition: $validate_input.success
  - from: transform_user
    to: save_user
  - from: save_user
    to: send_welcome_email
    condition: $save_user.success', '1.0.0', 'active');

-- Demo 3: Data Processing Pipeline
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('c3d4e5f6-a7b8-9012-cdef-345678901234', 'Daily Data Processing', 'ETL workflow for processing daily data feeds', 'metadata:
  name: "Daily Data Processing"
  version: "1.0.0"
  description: "ETL workflow for processing daily data feeds"
  tags: ["etl", "scheduled", "data-processing"]

triggers:
  - schedule:
      cron: "0 2 * * *"
      timezone: "UTC"

nodes:
  fetch_data:
    type: http
    method: GET
    url: "https://api.example.com/data/daily"
    headers:
      Authorization: "Bearer $env(API_TOKEN)"
    timeout: 30
    retry:
      max_attempts: 3
      delay_ms: 1000

  validate_data:
    type: validator
    rules:
      - field: records
        type: array
        required: true
        min_items: 1
      - field: timestamp
        type: string
        required: true

  transform_records:
    type: forEach
    items: $fetch_data.output.records
    operation:
      type: transformer
      mapping:
        id: $item.id
        name: $upper($item.name)
        value: $item.value * 1.1
        processed_at: $now()
        source: "daily_feed"

  aggregate_stats:
    type: transformer
    mapping:
      total_records: $len($transform_records.output)
      total_value: $sum($transform_records.output[*].value)
      avg_value: $avg($transform_records.output[*].value)
      processing_date: $now()

  save_results:
    type: parallel
    branches:
      - save_records:
          type: database
          operation: bulk_insert
          table: processed_records
          data: $transform_records.output
      - save_stats:
          type: database
          operation: insert
          table: daily_stats
          data: $aggregate_stats.output

  send_report:
    type: conditional
    condition: $save_results.success
    then:
      type: http
      method: POST
      url: "https://slack.com/api/chat.postMessage"
      headers:
        Authorization: "Bearer $env(SLACK_TOKEN)"
      body:
        channel: "#data-reports"
        text: "Daily processing complete: $aggregate_stats.output.total_records records processed"
    else:
      type: http
      method: POST
      url: "https://api.alerts.com/notify"
      body:
        alert_type: "etl_failure"
        message: "Daily data processing failed"

connections:
  - from: trigger
    to: fetch_data
  - from: fetch_data
    to: validate_data
  - from: validate_data
    to: transform_records
    condition: $validate_data.success
  - from: transform_records
    to: aggregate_stats
  - from: aggregate_stats
    to: save_results
  - from: save_results
    to: send_report', '1.0.0', 'active');

-- Demo 4: Error Handling Workflow
INSERT INTO workflows(id, name, description, definition, version, status)
  VALUES ('d4e5f6a7-b8c9-0123-defa-456789012345', 'Payment Processing with Error Handling', 'Demonstrates retry logic and error handling patterns', 'metadata:
  name: "Payment Processing with Error Handling"
  version: "1.0.0"
  description: "Demonstrates retry logic and error handling patterns"

triggers:
  - event:
      source: "payment-service"
      event_type: "payment.initiated"

nodes:
  validate_payment:
    type: validator
    rules:
      - field: amount
        type: number
        required: true
        minimum: 0.01
      - field: currency
        type: string
        required: true
        allowed_values: ["USD", "EUR", "GBP"]
      - field: customer_id
        type: string
        required: true

  process_payment:
    type: http
    method: POST
    url: "https://payment-gateway.com/api/charge"
    timeout: 30
    retry:
      max_attempts: 3
      delay_ms: 2000
      backoff_multiplier: 2
      max_delay_ms: 10000
    headers:
      Authorization: "Bearer $env(PAYMENT_API_KEY)"
    body:
      amount: $validate_payment.data.amount
      currency: $validate_payment.data.currency
      customer: $validate_payment.data.customer_id
      metadata:
        order_id: $trigger.data.order_id

  update_order:
    type: database
    operation: update
    table: orders
    condition: "id = $trigger.data.order_id"
    data:
      payment_status: "completed"
      payment_id: $process_payment.output.transaction_id
      paid_at: $now()

  notify_customer:
    type: parallel
    branches:
      - send_email:
          type: http
          method: POST
          url: "https://api.email.com/send"
          body:
            to: $trigger.data.customer_email
            subject: "Payment Successful"
            template: "payment_success"
            data:
              amount: $validate_payment.data.amount
              currency: $validate_payment.data.currency
      - send_sms:
          type: http
          method: POST
          url: "https://api.sms.com/send"
          condition: $trigger.data.customer_phone
          body:
            to: $trigger.data.customer_phone
            message: "Payment of $validate_payment.data.amount $validate_payment.data.currency received"

  handle_error:
    type: switch
    expression: $process_payment.error_code
    cases:
      - when: "insufficient_funds"
        then:
          type: http
          method: POST
          url: "https://api.email.com/send"
          body:
            to: $trigger.data.customer_email
            subject: "Payment Failed - Insufficient Funds"
            template: "payment_insufficient_funds"
      - when: "card_declined"
        then:
          type: database
          operation: update
          table: orders
          condition: "id = $trigger.data.order_id"
          data:
            payment_status: "declined"
            declined_reason: $process_payment.error_message
      - default:
          type: http
          method: POST
          url: "https://api.alerts.com/critical"
          body:
            alert_type: "payment_error"
            order_id: $trigger.data.order_id
            error: $process_payment.error_message

connections:
  - from: trigger
    to: validate_payment
  - from: validate_payment
    to: process_payment
    condition: $validate_payment.success
  - from: process_payment
    to: update_order
    condition: $process_payment.success
  - from: update_order
    to: notify_customer
    condition: $update_order.success
  - from: process_payment
    to: handle_error
    condition: $process_payment.error', '1.0.0', 'active');

-- Create some sample execution records
INSERT INTO executions(workflow_id, status, trigger_data, outputs, started_at, completed_at)
  VALUES ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'Completed', '{"manual": true}', '{"message": "Hello, World!", "timestamp": "2024-01-15T10:30:00Z"}', NOW() - INTERVAL '2 hours', NOW() - INTERVAL '2 hours' + INTERVAL '500 milliseconds'),
('b2c3d4e5-f6a7-8901-bcde-f23456789012', 'Failed', '{"email": "test@example.com", "name": "Test User", "password": "weak"}', '{}', NOW() - INTERVAL '1 hour', NOW() - INTERVAL '1 hour' + INTERVAL '200 milliseconds'),
('c3d4e5f6-a7b8-9012-cdef-345678901234', 'Running', '{"scheduled": true}', '{}', NOW() - INTERVAL '5 minutes', NULL);
