output "id" {
  value       = aws_apigatewayv2_api.apg.id
  description = "The id of the api gateway"
}

output "execution_arn" {
  value       = aws_apigatewayv2_api.apg.execution_arn
  description = "The execution arn of the api gateway"
}

output "api_gateway" {
  value       = aws_apigatewayv2_api.apg
  description = "The api gateway"
}

output "api_gateway_stage" {
  value       = aws_apigatewayv2_stage.default_stage
  description = "The api gateway stage"
}
