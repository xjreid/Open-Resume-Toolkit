# Executed by the system Perl with a cleared environment and taint mode. Never
# execute the user-owned source: copy from one bounded regular-file descriptor,
# verify the protected copy, then invoke only that copy without a shell.
use strict;
use warnings;
$ENV{PATH} = '/usr/bin:/bin';
use Fcntl qw(O_RDONLY O_WRONLY O_CREAT O_EXCL O_NOFOLLOW O_NONBLOCK);
use File::Temp qw(tempdir);
use Digest::SHA;

die "administrator required\n" unless $> == 0;
die "invalid arguments\n" unless @ARGV == 3;
my ($source, $payload, $expected) = @ARGV;
($source) = $source =~ m{\A(/[^\x00\r\n]+)\z} or die "invalid source\n";
($payload) = $payload =~ m{\A(/[^\x00\r\n]+)\z} or die "invalid payload\n";
($expected) = $expected =~ /\A([a-f0-9]{64})\z/ or die "invalid digest\n";

sysopen(my $input, $source, O_RDONLY | O_NOFOLLOW | O_NONBLOCK)
    or die "cannot open helper\n";
my @metadata = stat($input);
die "invalid helper\n" unless -f $input && $metadata[3] == 1
    && $metadata[7] > 0 && $metadata[7] <= 16 * 1024 * 1024;
my $work = tempdir('ort-codex-root-XXXXXXXX', DIR => '/private/tmp', CLEANUP => 1);
($work) = $work =~ m{\A(/private/tmp/ort-codex-root-[A-Za-z0-9_]+)\z}
    or die "invalid protected directory\n";
chmod(0700, $work) == 1 or die "cannot protect directory\n";
my @directory = stat($work);
die "unprotected directory\n" unless $directory[4] == $> && ($directory[2] & 0777) == 0700;
my $helper = "$work/helper";
sysopen(my $output, $helper, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600)
    or die "cannot create protected helper\n";
my $digest = Digest::SHA->new(256);
my $total = 0;
while (1) {
    my $count = sysread($input, my $bytes, 65536);
    die "cannot read helper\n" unless defined $count;
    last unless $count;
    $total += $count;
    die "oversized helper\n" if $total > $metadata[7];
    $digest->add($bytes);
    my $offset = 0;
    while ($offset < $count) {
        my $written = syswrite($output, $bytes, $count - $offset, $offset);
        die "cannot copy helper\n" unless defined($written) && $written > 0;
        $offset += $written;
    }
}
close($input) or die "cannot close source\n";
close($output) or die "cannot close copy\n";
die "helper verification failed\n" unless $total == $metadata[7]
    && $digest->hexdigest eq $expected;
chmod(0500, $helper) == 1 or die "cannot protect helper\n";
system { $helper } $helper, $payload;
die "helper execution failed\n" if $? != 0;
