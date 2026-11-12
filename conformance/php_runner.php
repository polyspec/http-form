<?php

// Runs contracts/fields.json through the PHP extension as hyper's runners do and prints the actual results as JSON.
// A urlencoded text is the UTF-8 encoding of its JSON string; a multipart type and body are text whose characters
// are their bytes. The runner holds no rule: it calls the extension and records what it returns.

declare(strict_types=1);

use Polyspec\Forms;

function bytes(string $text): string
{
    return mb_convert_encoding($text, 'ISO-8859-1', 'UTF-8');
}

function text(string $bytes): string
{
    return mb_convert_encoding($bytes, 'UTF-8', 'ISO-8859-1');
}

function fieldsOf(?array $pairs): ?array
{
    return $pairs === null ? null : array_map(fn (array $pair) => [$pair[0], $pair[1]], $pairs);
}

function filesOf(?array $pairs, bool $size = false): ?array
{
    if ($pairs === null) {
        return null;
    }
    $out = [];
    foreach ($pairs as [, $items]) {
        foreach ($items as $item) {
            $entry = [
                'name' => $item->name(),
                'filename' => $item->filename(),
                'contentType' => $item->contentType(),
                'content' => text($item->content()),
            ];
            if ($size) {
                $entry['size'] = $item->size();
            }
            $out[] = $entry;
        }
    }
    return $out;
}

function failure(callable $work): ?array
{
    try {
        $work();
        return null;
    } catch (\TypeError $error) {
        return [$error->getMessage(), 'type'];
    } catch (\InvalidArgumentException $error) {
        return [$error->getMessage(), 'value'];
    }
}

function urlencoded(array $case): array
{
    $pairs = Forms\parse_urlencoded($case['text']);
    return ['fields' => fieldsOf($pairs), 'names' => $pairs === null ? null : array_map(fn ($p) => $p[0], $pairs)];
}

function multipart(array $case): array
{
    $kind = bytes($case['type']);
    $body = bytes($case['body']);
    $whole = [fieldsOf(Forms\fields_from_body($kind, $body)), filesOf(Forms\files_from_body($kind, $body))];
    $result = $whole;
    $boundary = Forms\multipart_boundary($kind);
    if ($boundary !== null) {
        for ($at = 0; $at <= strlen($body); $at++) {
            $parts = Forms\parse_multipart_chunks([substr($body, 0, $at), substr($body, $at)], $boundary);
            $divided = [fieldsOf($parts === null ? null : $parts[0]), filesOf($parts === null ? null : $parts[1])];
            if ($divided !== $whole) {
                $result = $divided;
                break;
            }
        }
    }
    return ['fields' => $result[0], 'files' => $result[1]];
}

function streamed(array $case): array
{
    $kind = bytes($case['type']);
    $body = bytes($case['body']);
    $read = new Forms\RequestBody(
        $kind, $case['limit'], $case['stream'] === true, $case['valueLimit'], $case['partLimit'], $case['fileLimit'],
    );
    $scratch = sys_get_temp_dir() . '/http-forms-case-' . bin2hex(random_bytes(8));
    mkdir($scratch, 0700);
    $out = [];
    try {
        $stopped = false;
        for ($at = 0; $at < strlen($body) && !$stopped; $at += $case['chunk']) {
            $stopped = !$read->feed(substr($body, $at, $case['chunk']));
        }
        $held = $read->held();
        if ($read->streamed()) {
            $parts = $read->parts();
            $fields = $parts === null ? null : $parts[0];
            $files = $parts === null ? null : $parts[1];
        } else {
            $fields = $stopped ? null : Forms\fields_from_body($kind, $held);
            $files = $stopped ? null : Forms\files_from_body($kind, $held);
        }
        $directory = $read->directory();
        $count = fn () => $directory === null ? null : count(array_diff(scandir($directory), ['.', '..']));
        $out['stopped'] = $stopped;
        $out['exceeded'] = $read->exceeded();
        $out['held'] = text($held);
        $out['fields'] = fieldsOf($fields);
        $out['files'] = filesOf($files, true);
        $out['entries'] = $count();
        $pairs = $files ?? [];
        if ($case['move'] === 'none' || $pairs === []) {
            $out['moved'] = null;
        } else {
            $first = $pairs[0][1][0];
            $target = "{$scratch}/target";
            if ($case['move'] === 'exists') {
                file_put_contents($target, 'taken');
            }
            $failed = failure(fn () => $first->moveTo($case['move'] === 'type' ? 7 : $target));
            $after = null;
            if ($failed === null) {
                $content = text((string) file_get_contents($target));
                $failed = failure(fn () => $first->moveTo("{$scratch}/second"));
                $readFailure = failure(fn () => $first->content());
                $after = $readFailure === null ? null : $readFailure[0];
            } else {
                $content = text($first->content());
            }
            $out['moved'] = [
                'content' => $content,
                'left' => $count(),
                'again' => $failed[0] ?? null,
                'read' => $after,
                'class' => $failed[1] ?? null,
            ];
        }
        $read->remove();
        clearstatcache();
        $out['removed'] = $directory === null ? null : !is_dir($directory);
    } finally {
        $read->remove();
        array_map('unlink', glob("{$scratch}/*") ?: []);
        rmdir($scratch);
    }
    return $out;
}

$cases = json_decode((string) file_get_contents($argv[1]), true, 512, JSON_THROW_ON_ERROR);
$out = ['urlencoded' => [], 'multipart' => [], 'streamed' => []];
foreach ($cases['urlencoded'] as $case) {
    $out['urlencoded'][$case['id']] = urlencoded($case);
}
foreach ($cases['multipart'] as $case) {
    $out['multipart'][$case['id']] = multipart($case);
}
foreach ($cases['streamed'] as $case) {
    $out['streamed'][$case['id']] = streamed($case);
}
echo json_encode($out, JSON_THROW_ON_ERROR | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES), "\n";
