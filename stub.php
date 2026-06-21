<?php

namespace FluentPhp
{
    class Exception extends \Exception {}

    class ParserException extends Exception
    {
        /**
         * @return array<array{line: int, col: int, source: string}>
         */
        public function getErrors(): array {}
    }

    class ResolverException extends Exception
    {
        /**
         * @return array<string>
         */
        public function getErrors(): array {}
    }

    class FluentBundle
    {
        /**
         * @throws Exception if the language identifier is invalid
         */
        public function __construct(string $langCode)
        {
        }

        /**
         * @throws ParserException if the FTL source contains syntax errors
         * @throws Exception if any entry in the resource duplicates an existing one
         */
        public function addResource(string $resource): void
        {
        }

        /**
         * @param callable(): mixed $callable
         * @throws Exception if a function with that name is already registered
         */
        public function addFunction(string $name, callable $callable): void
        {
        }

        /**
         * @param array<string, mixed> $parameters
         * @throws Exception if the message is not found or has no value, or an argument type is unsupported
         * @throws ResolverException if the pattern references undefined variables or functions
         */
        public function formatPattern(string $messageId, array $parameters): string
        {
        }

        public function hasMessage(string $messageId): bool
        {
        }
    }
}
