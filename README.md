https://learn.microsoft.com/en-us/office/troubleshoot/office-suite-issues/fails-embedding-adobe-opentype-font

You cannot embed an Adobe OpenType font in a document in an Office program

qpdf svg.pdf --stream-data=uncompress --decode-level=all --normalize-content=n --qdf svg-out.pdf

qpdf font.pdf --stream-data=uncompress --decode-level=all --normalize-content=n --qdf font-out.pdf

https://pdfcrowd.com/inspect-pdf/

CID Type 0C (OT)

pdffonts font.pdf
pdfdetach -saveall font.pdf

12 dict begin

begincmap

%!PS-Adobe-3.0 Resource-CMap
%%DocumentNeededResources: procset CIDInit
%%IncludeResource: procset CIDInit

/CIDSystemInfo 3 dict dup begin
/Registry (FontSpecific) def
/Ordering (HEIGIDGCBAAHFGBHAEFHCBHGAJHCJDHF) def
/Supplement 0 def
end def

/CMapName /FontSpecific-HEIGIDGCBAAHFGBHAEFHCBHGAJHCJDHF def
/CMapVersion 1 def
/CMapType 2 def
/WMode 0 def

1 begincodespacerange
<0000> <FFFF>
endcodespacerange
4 beginbfchar
<0000> <0020>
<0001> <5287>
<0002> <6F22>
<0003> <7684>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end

```
CHAR CODE IS 26085 65e5
Some(
    Format4(
        CmapSubtableFormat4 {
            language: 0,
            end_codes: [
                32,
                160,
                12103,
                26085,
                26412,
                65535,
            ],
            start_codes: [
                32,
                160,
                12103,
                26085,
                26412,
                65535,
            ],
            id_deltas: [
                -31,
                -159,
                -12101,
                -26083,
                -26409,
                1,
            ],
            id_range_offsets: [
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            glyph_id_array: [],
        },
    ),
)
Ok(Some(2))
CHAR CODE IS 26412 672c
Some(
    Format4(
        CmapSubtableFormat4 {
            language: 0,
            end_codes: [
                32,
                160,
                12103,
                26085,
                26412,
                65535,
            ],
            start_codes: [
                32,
                160,
                12103,
                26085,
                26412,
                65535,
            ],
            id_deltas: [
                -31,
                -159,
                -12101,
                -26083,
                -26409,
                1,
            ],
            id_range_offsets: [
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            glyph_id_array: [],
        },
    ),
)
Ok(Some(3))
```

        let cmap_content = r##"%!PS-Adobe-3.0 Resource-CMap

%%DocumentNeededResources: ProcSet (CIDInit)
%%IncludeResource: ProcSet (CIDInit)
%%BeginResource: CMap (FontSpecific-CMap)
%%Title: (FontSpecific-CMap Adobe Identity 0)
%%Version: 1.0
%%EndComments

/CIDInit /ProcSet findresource begin

12 dict begin

begincmap

/CMapName /FontSpecific-CMap def
/CMapType 1 def

/CIDSystemInfo 3 dict dup begin
/Registry (Adobe) def
/Ordering (Identity) def
/Supplement 0 def
end def

/WMode 0 def

1 begincodespacerange
<0000> <FFFF>
endcodespacerange

7 begincidchar
<0001> 1
<0002> 34
<0003> 35
<0004> 36
<0005> 37
<0006> 20616
<0007> 21156
endcidchar

endcmap
CMapName currentdict /CMap defineresource pop
end
end
"##;

Adobe Acrobat / Safari / Preview.app (Mac)
