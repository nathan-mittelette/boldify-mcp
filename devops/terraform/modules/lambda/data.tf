data "archive_file" "lambda_zip" {
  type        = "zip"
  source_dir  = "../../${var.lambda.source}"
  output_path = "${path.module}/${var.lambda.output}/${var.lambda.name}.zip"
}
