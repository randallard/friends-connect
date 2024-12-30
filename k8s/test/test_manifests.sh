#!/bin/bash
set -e

# Check if files exist
if [ ! -f k8s/base/deployment.yaml ] || [ ! -f k8s/base/service.yaml ]; then
    echo "Error: Required YAML files not found"
    exit 1
fi

# Basic YAML syntax validation
echo "Testing deployment.yaml..."
if ! grep -q "apiVersion: apps/v1" k8s/base/deployment.yaml; then
    echo "✗ deployment.yaml missing apiVersion"
    exit 1
fi

if ! grep -q "kind: Deployment" k8s/base/deployment.yaml; then
    echo "✗ deployment.yaml missing kind"
    exit 1
fi

echo "✓ deployment.yaml basic validation passed"

echo "Testing service.yaml..."
if ! grep -q "apiVersion: v1" k8s/base/service.yaml; then
    echo "✗ service.yaml missing apiVersion"
    exit 1
fi

if ! grep -q "kind: Service" k8s/base/service.yaml; then
    echo "✗ service.yaml missing kind"
    exit 1
fi

echo "✓ service.yaml basic validation passed"

# Test if container port matches service targetPort
CONTAINER_PORT=$(grep -A1 "ports:" k8s/base/deployment.yaml | grep "containerPort:" | tr -d ' ' | cut -d':' -f2)
TARGET_PORT=$(grep "targetPort:" k8s/base/service.yaml | tr -d ' ' | cut -d':' -f2)

if [ "$CONTAINER_PORT" = "$TARGET_PORT" ]; then
    echo "✓ Container port matches service targetPort"
else
    echo "✗ Container port ($CONTAINER_PORT) doesn't match service targetPort ($TARGET_PORT)"
    exit 1
fi

echo "All tests passed!"