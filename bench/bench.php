<?php
// Times hyper's PHP reader and the extension on the bodies of generate.py. Arguments: the directory of hyper's PHP
// sources (packages/server/php/src) and the directory of the bodies.
foreach (['ArgumentCount','NamedValues','Fields','Files','UploadedFile','MultipartReader','Multipart','RequestBody'] as $f) require "{$argv[1]}/$f.php";
use Polyspec\Hyper;
use Polyspec\Forms;
$kind = file_get_contents("{$argv[2]}/type.txt"); $boundary = explode('=', $kind, 2)[1];
$text = file_get_contents("{$argv[2]}/form.txt"); $form = file_get_contents("{$argv[2]}/form.bin"); $large = file_get_contents("{$argv[2]}/large.bin");
const CHUNK = 65536; const BIG = 1 << 40;
function median(callable $w, int $n): float { $t = []; for ($i = 0; $i < $n; $i++) { $s = hrtime(true); $w(); $t[] = (hrtime(true) - $s) / 1e6; } sort($t); return $t[intdiv($n, 2)]; }
function hyperStream(string $kind, string $d): void { $b = new Hyper\RequestBody($kind, BIG, true, BIG, BIG, BIG); for ($i = 0; $i < strlen($d); $i += CHUNK) $b->feed(substr($d, $i, CHUNK)); if ($b->parts() === null) throw new Exception('no parts'); $b->remove(); }
function coreStream(string $kind, string $d): void { $b = new Forms\RequestBody($kind, BIG, true, BIG, BIG, BIG); for ($i = 0; $i < strlen($d); $i += CHUNK) $b->feed(substr($d, $i, CHUNK)); if ($b->parts() === null) throw new Exception('no parts'); $b->remove(); }
$out = [
 'urlencoded 1000 pairs' => [median(fn () => Hyper\Fields::parse($text), 300), median(fn () => Forms\parse_urlencoded($text), 300)],
 'multipart form, whole' => [median(fn () => Hyper\Multipart::read($form, $boundary), 300), median(fn () => Forms\parse_multipart_chunks([$form], $boundary), 300)],
 'multipart form, streamed' => [median(fn () => hyperStream($kind, $form), 100), median(fn () => coreStream($kind, $form), 100)],
 '50 MiB attachment, streamed' => [median(fn () => hyperStream($kind, $large), 7), median(fn () => coreStream($kind, $large), 7)],
];
echo json_encode(['php' => $out, 'version' => PHP_VERSION, 'opcache_jit' => ini_get('opcache.jit')]), "\n";
