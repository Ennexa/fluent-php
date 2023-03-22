<?php

namespace FluentPhp
{
    class FluentBundle
    {
        public function __construct(string $langCode)
        {
        }

        public function addResource(string $resource): void
        {
        }

        /**
         * @param callable(): mixed $callable
         */
        public function addFunction(string $name, callable $callable): void
        {
        }

        /**
         * @param array<string, mixed> $parameters
         */
        public function formatPattern(string $messageId, array $parameters): string
        {
        }
    }
}

