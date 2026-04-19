#!/usr/bin/env perl
# Emit exif.json, gps.json, png.json for exifinf-rs build.rs (pure data only).
use strict;
use warnings;
use JSON::PP ();
use FindBin ();
use File::Spec ();
use File::Path qw(make_path);

BEGIN {
    my $repo = File::Spec->catdir($FindBin::Bin, '..', '..');
    unshift @INC, File::Spec->catdir($repo, 'lib');
}

use Image::ExifTool::Exif ();
use Image::ExifTool::GPS ();
use Image::ExifTool::PNG ();

my $repo = File::Spec->catdir($FindBin::Bin, '..', '..');
my $outdir = File::Spec->catdir($repo, 'rust', 'exifinf-rs', 'data');
make_path($outdir);

my $json = JSON::PP->new->canonical(1)->pretty(1)->ascii(1);

write_json(File::Spec->catfile($outdir, 'exif.json'),  dump_exif_main());
write_json(File::Spec->catfile($outdir, 'gps.json'),  dump_hash_table(\%Image::ExifTool::GPS::Main, 'gps'));
write_json(File::Spec->catfile($outdir, 'png.json'),  dump_png());

sub write_json {
    my ($path, $data) = @_;
    open my $fh, '>:utf8', $path or die "write $path: $!";
    print $fh $json->encode($data);
    close $fh;
}

sub dump_exif_main {
    return dump_hash_table(\%Image::ExifTool::Exif::Main, 'exif');
}

sub dump_png {
    my %out = (
        chunks => dump_png_main_chunks(),
        textual => dump_png_textual(),
    );
    return \%out;
}

sub dump_png_main_chunks {
    my %main;
    for my $k (keys %Image::ExifTool::PNG::Main) {
        next if $k !~ /^[a-zA-Z]{4}$/;    # only 4-letter chunk types
        my $v = $Image::ExifTool::PNG::Main{$k};
        my $e = normalize_tag_entry($k, $v, 'png_chunk');
        $main{$k} = $e if $e;
    }
    return \%main;
}

sub dump_png_textual {
    my %td;
    for my $k (keys %Image::ExifTool::PNG::TextualData) {
        next if ref($k) || $k =~ /^(PROCESS_PROC|WRITE_PROC|CHECK_PROC|NOTES|GROUPS|LANG_INFO|WRITABLE|PREFERRED)$/;
        my $v = $Image::ExifTool::PNG::TextualData{$k};
        my $e = normalize_tag_entry($k, $v, 'png_text');
        next unless $e;
        $td{$k} = $e;
    }
    return \%td;
}

sub dump_hash_table {
    my ($href, $ctx) = @_;
    my %out;
    for my $k (sort {
            my $an = ($a =~ /^-?\d+$/);
            my $bn = ($b =~ /^-?\d+$/);
            ($an && $bn) ? ($a <=> $b) : (($an <=> $bn) || $a cmp $b);
        } keys %$href) {
        next if $k =~ /^(GROUPS|WRITE_PROC|CHECK_PROC|NOTES|VARS|LANG_INFO|PROCESS_PROC|WRITE_GROUP|SET_GROUP1|WRITABLE|PREFERRED|READ_PROC|FIRST_ENTRY)$/;
        my $v = $href->{$k};
        my $hex = format_key($k, $ctx);
        next unless defined $hex;
        my $e = normalize_tag_entry($k, $v, $ctx);
        $out{$hex} = $e if $e;
    }
    return \%out;
}

sub format_key {
    my ($k, $ctx) = @_;
    if ($ctx eq 'png_chunk' || $ctx eq 'png_text') {
        return $k if !ref($k);
        return undef;
    }
    return sprintf('0x%04x', $k) if !ref($k) && $k =~ /^-?\d+$/;
    return undef;
}

sub normalize_tag_entry {
    my ($tag_id, $info, $ctx) = @_;
    my $num_id = (!ref($tag_id) && $tag_id =~ /^-?\d+$/) ? 0 + $tag_id : undef;

    # MakerNotes tag: opaque blob in Rust (value is \@MakerNotes::Main)
    if ($ctx eq 'exif' && defined $num_id && $num_id == 0x927c) {
        return {
            name        => 'MakerNote',
            writable    => 'undef',
            group1      => 'ExifIFD',
            print_conv  => undef,
            sub_directory => 'MakerNotes',
        };
    }

    my $href;
    if (ref($info) eq 'ARRAY') {
        $href = first_variant($info);
    } elsif (ref($info) eq 'HASH') {
        $href = $info;
    } elsif (!ref($info)) {
        $href = { Name => $info };
    } else {
        return undef;
    }

    return undef unless $href && ref($href) eq 'HASH';

    my $name = $href->{Name} // $href->{name};
    if ((!defined $name || $name eq '') && $ctx eq 'png_text' && !ref($tag_id)) {
        $name = $tag_id;
    }
    return undef unless defined $name && $name ne '';

    my $writable = $href->{Writable};
    if ($ctx =~ /^png/) {
        if (defined $writable && (ref($writable) || $writable =~ /^\d+$/)) {
            $writable = undef;    # ExifTool boolean writable flag, not TIFF format
        }
        $writable = 'string' if !defined $writable && $ctx eq 'png_text';
    }

    my $g1 = $href->{Groups} && ref($href->{Groups}) eq 'HASH'
        ? $href->{Groups}{1}
        : undef;
    if (!defined $g1) {
        $g1 = default_group1($ctx);
    }

    my $pc = extract_print_conv($href->{PrintConv});
    my $sd = extract_sub_directory($href->{SubDirectory}, $href);

    return {
        name          => $name,
        writable      => $writable,
        group1        => $g1,
        print_conv    => $pc,
        sub_directory => $sd,
    };
}

sub default_group1 {
    my ($ctx) = @_;
    return 'IFD0' if $ctx eq 'exif';
    return 'GPS' if $ctx eq 'gps';
    return 'PNG' if $ctx eq 'png_chunk';
    return 'PNG' if $ctx eq 'png_text';
    return 'PNG';
}

sub first_variant {
    my ($arr) = @_;
    for my $elt (@$arr) {
        next unless $elt && ref($elt) eq 'HASH';
        my $n = $elt->{Name} // '';
        next if $n =~ /^MakerNote/;    # skip maker-note variants when picking EXIF table row
        return $elt;
    }
    return $arr->[0] if @$arr && ref($arr->[0]) eq 'HASH';
    return undef;
}

sub extract_print_conv {
    my ($pc) = @_;
    return undef unless defined $pc;
    return undef if ref($pc) eq 'CODE';
    if (ref($pc) eq 'HASH') {
        return undef if exists $pc->{OTHER} || exists $pc->{BITMASK};
        return undef unless is_pure_print_map($pc);
        my %out;
        for my $k (sort keys %$pc) {
            my $ks = print_key_to_str($k);
            return undef unless defined $ks;
            my $v = $pc->{$k};
            return undef if ref($v);
            $out{"$ks"} = "$v";
        }
        return \%out;
    }
    return undef;
}

sub print_key_to_str {
    my ($k) = @_;
    return undef if ref($k);
    if ($k eq 'OTHER' || $k eq 'BITMASK') {
        return undef;
    }
    if ($k =~ /^0x[0-9a-fA-F]+$/) {
        return hex($k);
    }
    if ($k =~ /^-?\d+$/) {
        return 0 + $k;
    }
    return "$k";
}

sub is_pure_print_map {
    my ($href) = @_;
    for my $k (keys %$href) {
        return 0 if $k eq 'OTHER' || $k eq 'BITMASK';
        my $v = $href->{$k};
        my $r = ref($v);
        return 0 if $r eq 'CODE' || $r eq 'Regexp' || $r eq 'HASH' || $r eq 'ARRAY';
        return 0 if $r && $r ne '';
    }
    return 1;
}

sub extract_sub_directory {
    my ($sd, $tag) = @_;
    return undef unless $sd && ref($sd) eq 'HASH';
    my $dn = $sd->{DirName};
    if ($dn) {
        return 'ExifIFD'   if $dn eq 'ExifIFD';
        return 'GPS'       if $dn eq 'GPS';
        return 'InteropIFD' if $dn eq 'InteropIFD';
        return 'SubIFD'    if $dn eq 'SubIFD';
    }
    my $tt = $sd->{TagTable};
    if ($tt && $tt =~ /::MakerNotes/) {
        return 'MakerNotes';
    }
    if ($sd->{Start} && defined $sd->{MaxSubdirs}) {
        return 'SubIFD';
    }
    if ($sd->{Start} && !$tt) {
        return 'SubIFD';
    }
    return undef;
}
