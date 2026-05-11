resource "aws_lambda_function" "lambda" {
  filename      = data.archive_file.lambda_zip.output_path
  function_name = "${local.resources_name}-${var.lambda.name}"
  description   = var.lambda.description
  role          = aws_iam_role.lambda_exec.arn
  handler       = var.lambda.handler
  memory_size   = var.lambda.memory
  timeout       = var.lambda.timeout
  architectures = ["x86_64"]
  tracing_config {
    mode = "Active"
  }
  layers           = ["arn:aws:lambda:eu-west-1:580247275435:layer:LambdaInsightsExtension:53"]
  source_code_hash = filebase64sha256(data.archive_file.lambda_zip.output_path)
  runtime          = var.lambda.runtime
}
