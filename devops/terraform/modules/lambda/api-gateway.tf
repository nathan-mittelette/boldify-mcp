resource "aws_apigatewayv2_integration" "lambda_integration" {
  api_id                 = var.api_gateway.id
  payload_format_version = "2.0"
  description            = var.lambda.description
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.lambda.invoke_arn
}

resource "aws_apigatewayv2_route" "routes" {
  for_each  = toset([for route in var.lambda.https : "${route.method} ${route.path}"])
  api_id    = var.api_gateway.id
  route_key = each.key
  target    = "integrations/${aws_apigatewayv2_integration.lambda_integration.id}"
}
