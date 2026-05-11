resource "aws_apigatewayv2_integration" "lambda_integration" {
  api_id                 = var.api_gateway.id
  payload_format_version = "2.0"
  description            = var.lambda.description
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.lambda.invoke_arn
}

resource "aws_apigatewayv2_route" "default_route" {
  api_id    = var.api_gateway.id
  route_key = "${var.lambda.http.method} ${var.lambda.http.path}"
  target    = "integrations/${aws_apigatewayv2_integration.lambda_integration.id}"
}
